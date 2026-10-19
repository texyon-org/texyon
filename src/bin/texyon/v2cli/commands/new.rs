use clap::Parser;
use std::env;
use std::path::PathBuf;
use texyon::{
    config::PersistentConfig, ctry, docmodel::WorkspaceCreatorExt, errors::Result, tt_note,
};
use texyon_docmodel::workspace::WorkspaceCreator;
use texyon_status_base::StatusBackend;

use crate::v2cli::{CommandCustomizations, TexyonCommand};

/// `init`: Initialize a document project in the current directory.
#[derive(Debug, Eq, PartialEq, Parser)]
pub struct InitCommand {
    /// Use this URL to find resource files instead of the default
    #[arg(long, short)]
    bundle: Option<String>,
}

impl TexyonCommand for InitCommand {
    fn customize(&self, _cc: &mut CommandCustomizations) {}

    fn execute(self, config: PersistentConfig, status: &mut dyn StatusBackend) -> Result<i32> {
        let path = env::current_dir()?;
        tt_note!(
            status,
            "creating new document in this directory ({})",
            path.display()
        );

        let wc = WorkspaceCreator::new(path);
        ctry!(
            wc.create_defaulted(&config, self.bundle);
            "failed to create the new TeXyon workspace"
        );
        Ok(0)
    }
}

/// `new`: Create a new document project
#[derive(Debug, Eq, PartialEq, Parser)]
pub struct NewCommand {
    /// The name of the document directory to create.
    #[arg(default_value = ".")]
    path: PathBuf,

    /// Use this URL to find resource files instead of the default
    #[arg(long, short)]
    bundle: Option<String>,
}

impl TexyonCommand for NewCommand {
    fn customize(&self, _cc: &mut CommandCustomizations) {}

    fn execute(self, config: PersistentConfig, status: &mut dyn StatusBackend) -> Result<i32> {
        tt_note!(
            status,
            "creating new document in directory `{}`",
            self.path.display()
        );

        let wc = WorkspaceCreator::new(self.path);
        ctry!(
            wc.create_defaulted(&config, self.bundle);
            "failed to create the new TeXyon workspace"
        );
        Ok(0)
    }
}
