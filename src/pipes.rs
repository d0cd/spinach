// use std::borrow::Cow;
// use std::cell::RefCell;
use std::fmt::Debug;
use std::sync::mpsc;

// use tokio::sync::mpsc;
// use tokio::sync::broadcast;

// use tokio::stream::Stream;

use crate::merge::Merge;
// use crate::semilattice::Semilattice;


pub trait UnaryFn<I> {
    type Output;

    fn call(&self, input: I) -> Self::Output;
}


pub trait Pipe {
    type Item;

    #[must_use]
    fn push(&mut self, item: &Self::Item) -> Result<(), String>;
}

pub trait MovePipe {
    type Item;

    #[must_use]
    fn push(&mut self, item: Self::Item) -> Result<(), String>;
}

impl<P: Pipe> MovePipe for P {
    type Item = P::Item;

    fn push(&mut self, item: Self::Item) -> Result<(), String> {
        Pipe::push(self, &item)
    }
}




pub struct NullPipe<T> {
    _phantom: std::marker::PhantomData<T>,
}
impl<T> NullPipe<T> {
    pub fn new() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }
}
impl<T> Pipe for NullPipe<T> {
    type Item = T;

    fn push(&mut self, _item: &Self::Item) -> Result<(), String> {
        Ok(())
    }
}


pub struct DebugPipe<P: Pipe>
where
    P::Item: Debug,
{
    tag: &'static str,
    next_pipe: P,
}
impl<P: Pipe> DebugPipe<P>
where
    P::Item: Debug,
{
    pub fn new(tag: &'static str, next_pipe: P) -> Self {
        Self {
            tag: tag,
            next_pipe: next_pipe,
        }
    }
}
impl<P: Pipe> Pipe for DebugPipe<P>
where
    P::Item: Debug,
{
    type Item = P::Item;

    fn push(&mut self, item: &Self::Item) -> Result<(), String> {
        println!("{}: {:?}", self.tag, &item);
        self.next_pipe.push(item)
    }
}




pub struct ClonePipe<P: MovePipe>
where
    P::Item: Clone,
{
    next_pipe: P,
}
impl<P: MovePipe> ClonePipe<P>
where
    P::Item: Clone,
{
    pub fn new(next_pipe: P) -> Self {
        Self {
            next_pipe: next_pipe,
        }
    }
}
impl<P: MovePipe> Pipe for ClonePipe<P>
where
    P::Item: Clone,
{
    type Item = P::Item;

    fn push(&mut self, item: &Self::Item) -> Result<(), String> {
        self.next_pipe.push(item.clone())
    }
}




pub struct LatticePipe<F: Merge, P: Pipe<Item = F::Domain>> {
    value: F::Domain,
    next_pipe: P,
}
impl<F: Merge, P: Pipe<Item = F::Domain>> LatticePipe<F, P> {
    pub fn new(bottom: F::Domain, next_pipe: P) -> Self {
        Self {
            value: bottom,
            next_pipe: next_pipe,
        }
    }
}
impl<F: Merge, P: Pipe<Item = F::Domain>> MovePipe for LatticePipe<F, P> {
    type Item = F::Domain;

    fn push(&mut self, item: Self::Item) -> Result<(), String> {
        F::merge_in(&mut self.value, item);
        self.next_pipe.push(&self.value)
    }
}


pub struct MpscPipe<T> {
    sender: mpsc::SyncSender<T>,
}
impl<T> MpscPipe<T> {
    pub fn create(sender: mpsc::SyncSender<T>) -> Self {
        Self {
            sender: sender,
        }
    }
}
impl<T> MovePipe for MpscPipe<T> {
    type Item = T;

    fn push(&mut self, item: T) -> Result<(), String> {
        self.sender.send(item)
            .map_err(|e| format!("{}", e))
    }
}
impl<T> Clone for MpscPipe<T> {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
        }
    }
}


pub struct SplitPipe<P: MovePipe>
where
    P::Item: Clone,
{
    pipe_receiver: mpsc::Receiver<P>,
    pipes: Vec<P>,
}
impl<P: Pipe> SplitPipe<P>
where
    P::Item: Clone,
{
    pub fn create() -> ( Self, MpscPipe<P> ) {
        let ( sender, receiver ) = mpsc::sync_channel(8);
        let inst = Self {
            pipe_receiver: receiver,
            pipes: Vec::new(),
        };
        let mpsc_pipe = MpscPipe::create(sender);
        ( inst, mpsc_pipe )
    }
}
impl<P: Pipe> Pipe for SplitPipe<P>
where
    P::Item: Clone,
{
    type Item = P::Item;

    fn push(&mut self, item: &Self::Item) -> Result<(), String> {
        while let Ok(new_pipe) = self.pipe_receiver.try_recv() {
            self.pipes.push(new_pipe);
        }
        let mut result = Ok(());

        self.pipes.drain_filter(|pipe| {
            let next_result = pipe.push(item);
            let remove = next_result.is_err(); // DANGER!!!! Errored pipes get removed!!
            result = std::mem::replace(&mut result, Ok(())).and(next_result); // Ugly to fight ownership.
            remove
        });
        result
    }
}


pub struct MapFilterPipe<T, F: for<'a> UnaryFn<&'a T, Output = Option<P::Item>>, P: MovePipe> {
    mapfilter: F,
    next_pipe: P,
    _phantom: std::marker::PhantomData<T>,
}
impl<T, F: for<'a> UnaryFn<&'a T, Output = Option<P::Item>>, P: MovePipe> MapFilterPipe<T, F, P> {
    pub fn new(mapfilter: F, next_pipe: P) -> Self {
        Self {
            mapfilter: mapfilter,
            next_pipe: next_pipe,
            _phantom: std::marker::PhantomData,
        }
    }
}
impl<T, F: for<'a> UnaryFn<&'a T, Output = Option<P::Item>>, P: MovePipe> Pipe for MapFilterPipe<T, F, P> {
    type Item = T;

    fn push(&mut self, item: &T) -> Result<(), String> {
        if let Some(item) = self.mapfilter.call(item) {
            self.next_pipe.push(item)
        }
        else {
            Ok(())
        }
    }
}



pub fn test() -> Result<(), String> {
    use std::collections::HashMap;
    use crate::merge::{ MapUnion, Max };


    // Key-getter for reading.
    struct ReadKey {
        key: &'static str,
        // _phantom: std::marker::PhantomData<&'a ()>,
    }
    impl ReadKey {
        pub fn new(key: &'static str) -> Self {
            Self {
                key: key,
                // _phantom: std::marker::PhantomData,
            }
        }
    }
    impl<'a> UnaryFn<&'a HashMap<&'static str, &'static str>> for ReadKey {
        type Output = Option<&'static str>;

        fn call(&self, input: &'a HashMap<&'static str, &'static str>) -> Self::Output {
            input.get(self.key).cloned()
        }
    }


    // Mapper for writing.
    struct KvToHashmap;
    impl<'a> UnaryFn<&'a ( &'static str, &'static str )> for KvToHashmap {
        type Output = Option<HashMap<&'static str, &'static str>>;

        fn call(&self, &( k, v ): &'a ( &'static str, &'static str )) -> Self::Output {
            let mut hashmap = HashMap::new();
            hashmap.insert(k, v);
            Some(hashmap)
        }
    }

    // Set up pipes.
    let ( write_pipe, mut read_pipe ) = SplitPipe::create();
    let write_pipe = LatticePipe::<MapUnion<HashMap<&'static str, Max<&'static str>>>, _>::new(HashMap::new(), write_pipe);
    let write_pipe = MapFilterPipe::new(KvToHashmap, write_pipe);
    let mut write_pipe = write_pipe;

    // Add first reader.
    let read_pipe_foo = NullPipe::new();
    let read_pipe_foo = DebugPipe::new("foo_0", read_pipe_foo);
    let read_pipe_foo = MapFilterPipe::new(ReadKey::new("foo"), read_pipe_foo);
    read_pipe.push(read_pipe_foo)?;

    // Add second reader.
    let read_pipe_foo = NullPipe::new();
    let read_pipe_foo = DebugPipe::new("xyz_0", read_pipe_foo);
    let read_pipe_foo = MapFilterPipe::new(ReadKey::new("xyz"), read_pipe_foo);
    read_pipe.push(read_pipe_foo)?;

    // Do first set of writes.
    MovePipe::push(&mut write_pipe, ( "foo", "bar" ))?;
    MovePipe::push(&mut write_pipe, ( "bin", "bag" ))?;

    // Add third reader.
    let read_pipe_foo = NullPipe::new();
    let read_pipe_foo = DebugPipe::new("foo_1", read_pipe_foo);
    let read_pipe_foo = MapFilterPipe::new(ReadKey::new("foo"), read_pipe_foo);
    read_pipe.push(read_pipe_foo)?;

    // Do second set of writes.
    MovePipe::push(&mut write_pipe, ( "foo", "baz" ))?;
    MovePipe::push(&mut write_pipe, ( "xyz", "zzy" ))?;

    // Add fourth reader.
    let read_pipe_foo = NullPipe::new();
    let read_pipe_foo = DebugPipe::new("foo_2", read_pipe_foo);
    let read_pipe_foo = MapFilterPipe::new(ReadKey::new("foo"), read_pipe_foo);
    read_pipe.push(read_pipe_foo)?;

    Ok(())
}

#[test]
pub fn run_test() {
    println!("{:?}", test());
}
