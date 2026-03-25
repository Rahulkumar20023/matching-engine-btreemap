

pub struct PriceLevel{
    pub head: Option<usize>,
    pub tail: Option<usize>,
}

impl PriceLevel{
    pub fn new()->Self{
        Self{
            head:None,
            tail:None
        }
    }

    pub fn is_empty(&self)->bool{
        self.head.is_none()
    }
}