use pico_args::Arguments;
use std::{ffi::OsStr, path::PathBuf};

use crate::Result;
#[derive(Debug)]
pub(crate) struct AppArgs {
    pub(crate) query_graph: std::path::PathBuf,
    pub(crate) data_graph: std::path::PathBuf,
}

pub(crate) fn main() -> Result<AppArgs> {
    let mut pargs = Arguments::from_env();

    fn as_path_buf(arg: &OsStr) -> Result<PathBuf> {
        Ok(arg.into())
    }

    let args = AppArgs {
        query_graph: pargs.value_from_os_str(["-q", "--query-graph"], as_path_buf)?,
        data_graph: pargs.value_from_os_str(["-d", "--data-graph"], as_path_buf)?,
    };

    Ok(args)
}
