use std::io;
use std::marker::PhantomData;
use std::net::{IpAddr, Ipv4Addr};
use sha2::{Sha256, Digest};

//todo redo
pub trait ChordSend<T>{


    async fn send(&self, message: T) -> io::Result<()>;

    async fn receive(&self)->Option<T>;

    fn get_sha_dimension()->usize;

    fn use_sha(){

    }
}

pub struct ChordNode<A, T> where A: ChordSend<T>
{
    pub id: [u8; 32],
    pub address: A,
    pub predecessor: A,
    pub finger_table: Vec<A>,
    phantom: PhantomData<T>,
}


impl<A, T> ChordNode<A, T> where A: ChordSend<T>{

    //todo
    pub fn new(ip_addr: IpAddr)->Self<A, T>{
        Self
    }

    pub fn sha_length(mut self)->Self<A, T>{
        Self
    }


    pub async fn send_to_successor(&self, message: T){
        println!("{:?}", self.finger_table[0].send(message).await)
    }

    pub async fn send_to_predecessor(&self, message: T){
        println!("{:?}", self.predecessor.send(message).await)
    }

    pub fn look_up_key(&self, object_key: T){

    }

    pub fn stabilize(){

    }

    pub fn join(&mut self, node_to_add: Self){

    }
}