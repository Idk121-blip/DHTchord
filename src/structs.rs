use digest::{Digest, Output};
use std::io;
use std::marker::PhantomData;
use std::ops::Deref;
use std::sync::Arc;

pub trait ChordSend<T>{
    async fn send(&self, message: T) -> io::Result<()>;
    //async fn send_self();
    async fn send_string(&self, message: &str) -> io::Result<()>;
    async fn receive(&self)->Option<T>;
    async fn receive_string(&self)->Option<String>;
    fn to_be_bytes(&self)->[u8];
}

pub struct ChordNode<A, T, D> where A: ChordSend<T>+AsRef<[u8]>, D: Digest+Clone
{
    pub id: Output<D>,
    pub address: Arc<A>, //todo: mutex?
    pub predecessor: Arc<Option<Arc<Self>>>, //todo: mutex?
    pub finger_table: Vec<Arc<Self>>,//todo: Forse dovrevve essere un Arc
    sha: D,
    phantom: PhantomData<T>,
}

impl<A, T, D> ChordNode<A, T, D> where A: ChordSend<T>+AsRef<[u8]>,  D: Digest+Clone{
    pub fn new(self_reference: A, sha: D) ->Self{
        let reference= Arc::new(self_reference);
        let id= sha.clone().chain_update(reference.deref()).finalize();
        ChordNode{
            id,
            address: reference.clone(),
            predecessor: Arc::new(None),
            finger_table: vec![],
            sha,
            phantom: Default::default(),
        }
    }

    pub async fn send_to_successor(&self, message: T){
        if self.finger_table.is_empty() {
            log::error!("Finger table is empty, no successor available");
            return;
        }
        log::debug!("Message sent to successor response: {:?}", self.finger_table[0].address.send(message).await)
    }

    pub async fn send_to_predecessor(&mut self, message: T){
        if let Some(predecessor) = self.predecessor.as_ref() {
            log::debug!("Message sent to successor response: {:?}", predecessor.address.send(message).await)
        }
        else{
            log::error!("No predecessor available");
            return;
        }
    }

    pub async fn insert_value(&self){

    }

    pub async fn look_up_key(&self, key: Box<Output<D>>)->Option<Arc<A>>{
        loop {
            //todo: remove this loop o mmake it iterative
            if self.id == *key {
                return Some(self.address.clone());
            }

            let mut next_hop = None;

            for finger in self.finger_table.iter().rev() { //todo binary search
                if finger.id < *key  {
                    next_hop = Some(finger.clone());
                    break;
                }
            }

            return if let Some(next) = next_hop {
                //routing
                if next.address.clone().send_string("Searching for: ").await.is_ok()
                {
                    log::debug!("Delegation key searching message sent successfully");
                } else {
                    log::error!("Error while trying to send delegation key searching message");
                }
                next.look_up_key(key).await
            } else {
                // Could not route further; return failure.
                None
            }
        }
    }

    pub async fn stabilize(){
        //send message to next node and if it doesn't respond in 10 second use
        // finger table to find the new next and update finger table
        //once the finger table is update send the death of the node
    }

    pub async fn join(&mut self, mut node_to_add: Self){
        let id= self.sha.clone().chain_update(&node_to_add.address.deref()).finalize();
        node_to_add.id = id.clone();
        node_to_add.sha=self.sha.clone();


        //todo: Once found

        match self.predecessor.clone().deref() {
            None => {}
            Some(x) => {
                if x.id<node_to_add.id && self.id> node_to_add.id{
                    node_to_add.predecessor = self.predecessor.clone();
                    let predecessor_ref= Arc::new(node_to_add);
                    self.predecessor = Arc::new(Some(predecessor_ref.clone()));
                    self.predecessor.as_mut()
                        .unwrap()
                        .update_finger_table(predecessor_ref, NodePosition::Next).await;
                }
            }
        }


    }



    async fn update_finger_table(&self, node: Arc<Self>, node_position: NodePosition){

    }

}
enum NodePosition{
    Next = 1,
    Previous= -1
}