use bytes::Bytes;
use std::path::PathBuf;
use tokio::sync::mpsc;

use super::{Error, Progress, Download, unpack};

use futures::{stream, Stream, StreamExt, TryStream, TryStreamExt};
pub async fn unpack_stream<F>(
    dir: PathBuf,
    url: String,
    unpack: F,
) -> Result<impl TryStream<Ok = Progress, Error = Error>, Error>
where
    F: Fn(
            mpsc::UnboundedReceiver<Bytes>,
            PathBuf,
            mpsc::UnboundedSender<u64>,
        ) -> Result<(), unpack::Error>
        + Send
        + 'static,
{
    let (task, response, tx, rx) = super::init_download(&dir, url, unpack).await?;
    let init = Download {
        decode_task: Some(task),
        bytes_decoded: rx,
        phase: Phase::Running,
        progress: Progress::from(&response),
        stream: response.bytes_stream(),
        dir,
        tx,
    };

    let stream = stream::try_unfold(init, state_machine);
    // this is needed as try_next needs Pin<TryStream> an TryStream is
    // not implemented for Pin<TryStream> this is due to trait aliasses
    // not yet being stable, and will not be a problem in the future.
    // this line of code can be removed when trait aliasses are stabalized
    // let mut stream = stream.into_stream().boxed();
    Ok(stream.into_stream().boxed())
}

pub enum Phase {
    Running,
    DlDone,
}

async fn state_machine<S: Stream<Item = Result<Bytes, reqwest::Error>> + Unpin>(
    mut state: Download<S>,
) -> Result<Option<(Progress, Download<S>)>, Error> {
    match state.phase {
        Phase::Running => {
            let done = state.download_and_unpack().await?;
            if done {
                state.phase = Phase::DlDone;
            }
            Ok(Some((state.progress.clone(), state)))
        }
        Phase::DlDone => {
            let done = state.unpack().await?;
            match done {
                false => Ok(Some((state.progress.clone(), state))),
                true => Ok(None),
            }
        }
    }
}

impl<S: Stream<Item = Result<Bytes, reqwest::Error>> + Unpin> Download<S> {
    async fn unpack(&mut self) -> Result<bool, Error> {
        match self.bytes_decoded.recv().await {
            Some(bytes) => {
                self.progress.decoded = bytes;
                Ok(false)
            }
            None => {
                self.decode_task
                    .take()
                    .expect("should always have ownership over the task")
                    .await
                    .expect("io error occured joining with the task")
                    .map_err(|e| Error::Unpacking(e))?;
                Ok(true)
            }
        }
    }

    async fn download_and_unpack(&mut self) -> Result<bool, Error> {
        use Error::*;

        let stream_next = tokio::select! {
            item = self.stream.next() => item,
            decoded = self.bytes_decoded.recv() => {
                self.progress.decoded = decoded.unwrap();
                return Ok(false)
            }
        };

        let item = match stream_next {
            Some(item) => item,
            None => return Ok(true),
        };

        let bytes = item.map_err(InvalidResponse)?;

        self.progress.downloaded += bytes.len() as u64;
        match self.tx.send(bytes) {
            Ok(_) => Ok(false),
            Err(_) => {
                let err = self
                    .decode_task
                    .take()
                    .expect("should always have ownership over the task")
                    .await
                    .expect("io error occured joining with the task")
                    .expect_err("task should return an error if sending failed");
                return Err(Error::Unpacking(err));
            }
        }
    }
}
