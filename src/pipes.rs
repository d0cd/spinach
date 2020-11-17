use std::cell::RefCell;
use std::fmt::Debug;

use tokio::sync::mpsc;
use tokio::sync::broadcast;

// use tokio::stream::Stream;

use crate::merge::Merge;
// use crate::semilattice::Semilattice;


pub trait Pipe {
    type Input;

    #[must_use]
    fn push(&mut self, item: Self::Input) -> Result<(), &'static str>;
}


pub struct DebugPipe<T: Debug> {
    _phantom: std::marker::PhantomData<T>,
}
impl <T: Debug> DebugPipe<T> {
    pub fn new() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }
}
impl <T: Debug> Pipe for DebugPipe<T> {
    type Input = T;

    fn push(&mut self, item: Self::Input) -> Result<(), &'static str> {
        println!("{:#?}", item);
        Ok(())
    }
}


pub struct ClonePipe<'a, T: Clone, P: Pipe<Input = T>> {
    next_pipe: P,
    _phantom: std::marker::PhantomData<&'a T>,
}
impl <'a, T: Clone, P: Pipe<Input = T>> ClonePipe<'a, T, P> {
    pub fn new(next_pipe: P) -> Self {
        Self {
            next_pipe: next_pipe,
            _phantom: std::marker::PhantomData,
        }
    }
}
impl <'a, T: Clone, P: Pipe<Input = T>> Pipe for ClonePipe<'a, T, P> {
    type Input = &'a T;

    fn push(&mut self, item: Self::Input) -> Result<(), &'static str> {
        self.next_pipe.push(item.clone())
    }
}


pub struct LatticePipe<'a, F: Merge> {
    value: &'a RefCell<F::Domain>,
}
impl <'a, F: Merge> LatticePipe<'a, F> {
    pub fn new(value: &'a RefCell<F::Domain>) -> Self {
        Self {
            value: value,
        }
    }
}
impl <'a, F: Merge> Pipe for LatticePipe<'a, F> {
    type Input = F::Domain;

    fn push(&mut self, item: Self::Input) -> Result<(), &'static str> {
        F::merge_in(&mut self.value.borrow_mut(), item);
        Ok(())
    }
}

pub struct LatticePipe2<F: Merge, P>
where
    for <'a> P: Pipe<Input = &'a F::Domain>,
{
    value: F::Domain,
    next_pipe: P,
}
impl <F: Merge, P> LatticePipe2<F, P>
where
    for <'a> P: Pipe<Input = &'a F::Domain>,
{
    pub fn new(value: F::Domain, next_pipe: P) -> Self {
        Self {
            value: value,
            next_pipe: next_pipe,
        }
    }
}
impl <F: Merge, P> Pipe for LatticePipe2<F, P>
where
    for <'a> P: Pipe<Input = &'a F::Domain>,
{
    type Input = F::Domain;

    fn push(&mut self, item: Self::Input) -> Result<(), &'static str> {
        F::merge_in(&mut self.value, item);
        self.next_pipe.push(&self.value)
    }
}


// pub struct AnnaWorker<F: Merge, P: Pipe<Input = F::Domain>> {
//     value: RefCell<F::Domain>,
//     pipes: Vec<P>,
// }
// impl <F: Merge, P: Pipe<Input = F::Domain>> AnnaWorker<F, P> {
//     pub fn new(bottom: F::Domain) -> Self {
//         Self {
//             value: RefCell::new(bottom),
//             pipes: Vec::new(),
//         }
//     }
// }


#[test]
pub fn test_stuff() {
    let pipe = DebugPipe::new();
    let mut pipe = ClonePipe::new(pipe);
    let items: Vec<usize> = vec![ 1, 2, 3, 4, 5 ];
    for item in &items {
        pipe.push(item);
    }
}

// pub struct MpscPipe<T> {
//     sender: mpsc::Sender<T>,
// }
// impl <T> MpscPipe<T> {
//     pub fn new(sender: mpsc::Sender<T>) -> Self {
//         Self {
//             sender: sender,
//         }
//     }
// }
// impl <T> Pipe for MpscPipe<T> {
//     type Input = T;

//     fn push(&mut self, item: T) -> Result<(), &'static str> {

//     }
// }
