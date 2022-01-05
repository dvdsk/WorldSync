#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
// use linux::download_java;


#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
use macos::download_java;


#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
use windows::download_java;


