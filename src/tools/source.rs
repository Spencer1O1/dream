use super::list::ListSourceFiles;
use super::read::ReadSourceFile;
use super::Tool;

pub fn lucid_tools() -> Vec<Box<dyn Tool>> {
    vec![
        Box::new(ListSourceFiles::lucid()),
        Box::new(ReadSourceFile::lucid()),
    ]
}

pub fn compose_tools() -> Vec<Box<dyn Tool>> {
    vec![
        Box::new(ListSourceFiles::compose()),
        Box::new(ReadSourceFile::compose()),
    ]
}
