
use crate::arena::slot::Slot;

pub struct Arena<T>{
    slots: Vec<Slot<T>>,
    free: Option<usize>,
}

impl<T> Arena<T>{
    pub fn new()->Self{
        Self{
            slots: Vec::new(),
            free:None
        }
    }

    pub fn insert(&mut self, value: T)->usize{
        if let Some(free)=self.free{
            let slot=&mut self.slots[free];
            self.free=slot.next_free;
            slot.value=Some(value);
            free
        }else{
            self.slots.push(Slot{
                value: Some(value),
                next_free: None,
            });
            self.slots.len()-1
        }
    }

    pub fn get(&self, idx: usize)->Option<&T>{
         self.slots.get(idx)?.value.as_ref()
    }
    
    pub fn get_mut(&mut self, idx: usize)->Option<&mut T>{
        self.slots.get_mut(idx)?.value.as_mut()
    }

    pub fn remove(&mut self, idx: usize){
        let slot=&mut self.slots[idx];
        slot.value=None;
        slot.next_free=self.free;
        self.free=Some(idx);
    }
}