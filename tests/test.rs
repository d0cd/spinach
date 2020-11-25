use std::collections::HashMap;

use spinach::pipes::{ SharedMovePipe, /*ExclMovePipe, SharedRefPipe,*/ ExclRefPipe };
use spinach::pipes::{ UnaryFn, SplitPipe, LatticePipe, NullPipe, DebugPipe, MapFilterPipe }; //MpscPipe };
use spinach::merge::{ MapUnion, Max };


#[tokio::test]
pub async fn test_basic() -> Result<(), String> {


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
    let ( write_pipe, readers_pipe ) = SplitPipe::create();
    let write_pipe = LatticePipe::<MapUnion<HashMap<&'static str, Max<&'static str>>>, _>::new(HashMap::new(), write_pipe);
    let write_pipe = MapFilterPipe::new(KvToHashmap, write_pipe);
    let mut write_pipe = write_pipe;

    // Read pipes are weird.
    // Add first reader (subscriber).
    let read_pipe_foo = NullPipe::new();
    let read_pipe_foo = DebugPipe::new("foo_0", read_pipe_foo);
    let read_pipe_foo = MapFilterPipe::new(ReadKey::new("foo"), read_pipe_foo);
    readers_pipe.push(read_pipe_foo).await;

    // Add second reader.
    let read_pipe_foo = NullPipe::new();
    let read_pipe_foo = DebugPipe::new("xyz_0", read_pipe_foo);
    let read_pipe_foo = MapFilterPipe::new(ReadKey::new("xyz"), read_pipe_foo);
    readers_pipe.push(read_pipe_foo).await;

    // Do first set of writes.
    ExclRefPipe::push(&mut write_pipe, &( "foo", "bar" )).await;
    ExclRefPipe::push(&mut write_pipe, &( "bin", "bag" )).await;

    // Add third reader.
    let read_pipe_foo = NullPipe::new();
    let read_pipe_foo = DebugPipe::new("foo_1", read_pipe_foo);
    let read_pipe_foo = MapFilterPipe::new(ReadKey::new("foo"), read_pipe_foo);
    readers_pipe.push(read_pipe_foo).await;

    // Do second set of writes.
    ExclRefPipe::push(&mut write_pipe, &( "foo", "baz" )).await;
    ExclRefPipe::push(&mut write_pipe, &( "xyz", "zzy" )).await;

    // Add fourth reader.
    let read_pipe_foo = NullPipe::new();
    let read_pipe_foo = DebugPipe::new("foo_2", read_pipe_foo);
    let read_pipe_foo = MapFilterPipe::new(ReadKey::new("foo"), read_pipe_foo);
    readers_pipe.push(read_pipe_foo).await;

    Ok(())
}