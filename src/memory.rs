// use crate::objects::{Object, ObjectInner};

// pub struct GarbageCollector {
//     objects: Vec<Object>,
// }

// impl GarbageCollector {
//     pub fn collect() {
//         #[cfg(feature = "debug_trace_gc")]
//         println!("-- gc begin");

//         #[cfg(feature = "debug_trace_gc")]
//         println!("-- gc end");
//     }

//     pub fn alloc(&mut self, object: ObjectInner) {
//         #[cfg(feature = "debug_trace_gc")]
//         println!("-- gc alloc {}", object.inner);

//         let object = Object {
//             is_marked: false,
//             inner: object,
//         };
//         self.objects.push(object);
//     }
// }
