// use digest::{Digest, Output};
// use std::marker::PhantomData;
// use std::sync::Arc;
//
// pub trait ChordSend<T>{
//     fn to_be_bytes(&self)->[u8];
// }
//
// enum NodePosition{
//     Next = 1,
//     Previous = -1
// }
//
// pub enum UserActions<T, K> {
//     Put(T, oneshot::Sender<String>),
//     GetByKey(K, oneshot::Sender<T>),
// }
//
// pub(crate) enum ServerMessages{
//
// }
//
// pub struct ChordNode<A, T, D> where A: ChordSend<T>+AsRef<[u8]>, D: Digest+Clone
// {
//     pub id: Output<D>,
//     pub address: Arc<A>, //todo: mutex?
//     pub predecessor: Arc<Option<Arc<Self>>>, //todo: mutex?
//     pub finger_table: Vec<Arc<Self>>,//todo: Forse dovrevve essere un Arc
//     sha: D,
//     phantom: PhantomData<T>,
// }
//
// impl<A, T, D> ChordNode<A, T, D> where A: ChordSend<T>+AsRef<[u8]>,  D: Digest+Clone{
//
// }
