use super::list::ListSourceFiles;
use super::read::ReadSourceFile;
use super::Tool;

pub fn tools() -> Vec<Box<dyn Tool>> {
    vec![Box::new(ListSourceFiles), Box::new(ReadSourceFile)]
}
