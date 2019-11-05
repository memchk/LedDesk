
// struct StridedChunks<'a, T> {
//     buffer: &'a [T],
//     chunk_size: usize,
//     stride: usize
// }

// impl<'a, T> StridedChunks<'a, T> {
//     pub fn new(buffer: &'a [T], stride: usize, chunk_size: usize) -> Self {
//         Self {
//             buffer: buffer,
//             stride: stride,
//             chunk_size: chunk_size
//         }
//     }
// }

// impl<'a, T> Iterator for StridedChunks<'a, T> {
//     type Item = &'a [T];
//     fn next(&mut self) -> Option<Self::Item> {
//         if self.chunk_size <= self.buffer.len() {
//             let subslice = &self.buffer[..self.chunk_size];
//             let advance_amount = std::cmp::min(self.buffer.len(), self.stride);
//             self.buffer = &self.buffer[..advance_amount];
//             Some(subslice)
//         } else {
//             None
//         }
//     }
// }