use super::list_foo::ListFooFiles;
use super::read_foo::ReadFooFile;
use super::Tool;

pub fn lucid_tools() -> Vec<Box<dyn Tool>> {
    vec![
        Box::new(ListFooFiles::lucid()),
        Box::new(ReadFooFile::lucid()),
    ]
}

pub fn compose_tools() -> Vec<Box<dyn Tool>> {
    vec![
        Box::new(ListFooFiles::compose()),
        Box::new(ReadFooFile::compose()),
    ]
}
