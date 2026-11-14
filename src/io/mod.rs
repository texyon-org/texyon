// Copyright 2016-2021 the Tectonic Project
// Licensed under the MIT License.

//! Extensions to Tectonic’s pluggable I/O backend.

// These used to live here. They moved down into `texyon_io_base` when the
// live-preview engine became its own crate, because both it and this one need
// them and nothing about them was specific to the driver.

pub use texyon_io_base::{
    InputFeatures, InputHandle, InputOrigin, IoProvider, OpenResult, OutputHandle,
    digest::{self, DigestData},
    filesystem::{FilesystemIo, FilesystemPrimaryInputIo},
    format_cache, memory,
    memory::MemoryIo,
    normalize_tex_path,
    stack::IoStack,
    stdstreams::GenuineStdoutIo,
    testing, try_open_file,
};
