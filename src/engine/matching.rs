use std::collections::{BTreeMap, HashMap};

use crate::{
    arena::arena::Arena,
    orderbook::{book_node::BookNode, price_level::PriceLevel},
    types::{order_id::OrderId, price::Price, qty::Qty, side::Side},
};
use crate::orderbook::orderbook::OrderBook;
use crate::types::order::{Order,OrderType};
use crate::engine::trade::Trade;


pub struct MatchingEngine{
    pub book: OrderBook,
    pub arena: Arena<BookNode>,
    pub order_index: HashMap<OrderId, usize>,
    pub order_store: HashMap<OrderId, Order>,
}

impl MatchingEngine{
    pub fn new()->Self{
        Self{
            book: OrderBook::new(),
            arena: Arena::new(),
            order_index: HashMap::new(),
            order_store: HashMap::new(),
        }
    }

    pub fn process(&mut self, order: Order) -> Vec<Trade> {
        //let is_limit = matches!(order.order_type, OrderType::Limit);

        self.order_store.insert(order.id, order.clone());
        

        match order.order_type {
            OrderType::Market => self.process_market(order),
            OrderType::Limit  => self.process_limit(order),
        }
    }


    pub fn process_market(&mut self, mut order: Order)->Vec<Trade>{
        let mut trades=Vec::new();

        loop{
            if order.qty.0==0{
                break;
            }

            //Finding best available price
            let best_price=match order.side{
                Side::Buy=>{
                    let node=self.book.asks.iter().next(); //Option<(&Price, &PriceLevel)>
                    match node{
                        Some((price,_))=>price.clone(),
                        None=>{
                            break; //No more asks available
                        }
                    }

                },
                Side::Sell=>{
                    let node=self.book.bids.iter().next_back(); //Option<(&Price, &PriceLevel)>
                    match node{
                        Some((price,_))=>price.clone(),
                        None=>{
                            break; //No more asks available
                        }
                    }
                }
            };

            let new_trades=self.match_level(best_price,&mut order);

            if new_trades.is_empty(){
                break;
            }

            trades.extend(new_trades);

        }
        trades
    }

    fn process_limit(&mut self, mut order: Order)->Vec<Trade>{
        let mut trades=Vec::new();

        loop{
            if order.qty.0==0{
                break;
            }

            let best_price=match order.side{
                Side::Buy=>{
                    let node=self.book.asks.iter().next(); //Option<(&Price, &PriceLevel)>
                    match node{
                        Some((price,_))=>price.clone(),
                        None=>{
                            break; //No more asks available
                        }
                    }

                },
                Side::Sell=>{
                    let node=self.book.bids.iter().next_back(); //Option<(&Price, &PriceLevel)>
                    match node{
                        Some((price,_))=>price.clone(),
                        None=>{
                            break; //No more asks available
                        }
                    }
                }
            };

            let crosses=match order.side{
                Side::Buy  => best_price <= order.price,
                Side::Sell => best_price >= order.price,

            };

            if !crosses{
                break;
            }

            //Match againist this level
            let new_trades=self.match_level(best_price,&mut order);

            if new_trades.is_empty(){
                break;
            }
            trades.extend(new_trades);  //new_trades ek Vec<Trade>h,l iisiye extend kiye h ppus ni
        }

        //If there is remaining qty, add to orderbook
        if order.qty.0>0{
            self.rest(order);
        }
        trades
    }

    fn rest(&mut self, order:Order){
        let node=BookNode{
            order_id: order.id,
            remaining: order.qty,
            price: order.price,
            side: order.side,
            prev:None,
            next:None,
        };


        let idx=self.arena.insert(node);

        self.order_index.insert(order.id, idx); 

        let levels=match order.side{
            Side::Buy=>&mut self.book.bids,
            Side::Sell=>&mut self.book.asks,
        };

        let level=levels.entry(order.price).or_insert(PriceLevel::new());

        if let Some(tail)=level.tail{
            let tail_node=self.arena.get_mut(tail).unwrap();
            tail_node.next=Some(idx);
            let new_node=self.arena.get_mut(idx).unwrap();
            new_node.prev=Some(tail);
            level.tail=Some(idx);
        }else{
            level.head=Some(idx);
            level.tail=Some(idx);
        }
    }

   fn match_level(&mut self, price: Price, order: &mut Order) -> Vec<Trade> {
        let mut trades = Vec::new();

        let levels = match order.side {
            Side::Buy => &mut self.book.asks,
            Side::Sell => &mut self.book.bids,
        };

        let mut level_empty = false;

        {
            // Borrow exactly one price level
            let level = levels.get_mut(&price).unwrap();

            while let Some(head) = level.head {
                if order.qty.0 == 0 {
                    break;
                }

                // ---- Stage 1: touch only the head node ----
                let (next, filled_order_id, traded) = {
                    let node = self.arena.get_mut(head).unwrap();

                    let traded = node.remaining.0.min(order.qty.0);
                    node.remaining.0 -= traded;
                    order.qty.0 -= traded;

                    trades.push(Trade {
                        buy: if order.side == Side::Buy { order.id } else { node.order_id },
                        sell: if order.side == Side::Sell { order.id } else { node.order_id },
                        price,
                        qty: Qty(traded),
                    });

                    if node.remaining.0 == 0 {
                        (node.next, Some(node.order_id), traded)
                    } else {
                        (None, None, traded)
                    }
                };

                // ---- Stage 2: if fully filled, unlink + delete ----
                if let Some(order_id) = filled_order_id {
                    level.head = next;

                    if let Some(n) = next {
                        self.arena.get_mut(n).unwrap().prev = None;
                    } else {
                        level.tail = None;
                    }

                    self.order_index.remove(&order_id);
                    self.arena.remove(head);
                }
            }

            level_empty = level.head.is_none();
        }

        // Safe to remove empty price level
        if level_empty {
            levels.remove(&price);
        }

        trades
    }

    pub fn cancel(&mut self, id: OrderId) -> bool {
    // 1) Find live node
        let node_index = match self.order_index.get(&id) {
            Some(i) => *i,
            None => return false, // not live (already filled or cancelled)
        };

        // 2) Get node data before removing
        let node = self.arena.get(node_index).unwrap();
        let price = node.price;
        let side  = node.side;
        let prev  = node.prev;
        let next  = node.next;

        // 3) Unlink from price level
        let levels = match side {
            Side::Buy => &mut self.book.bids,
            Side::Sell => &mut self.book.asks,
        };

        let level = levels.get_mut(&price).unwrap();

        if let Some(p) = prev {
            self.arena.get_mut(p).unwrap().next = next;
        } else {
            level.head = next;
        }

        if let Some(n) = next {
            self.arena.get_mut(n).unwrap().prev = prev;
        } else {
            level.tail = prev;
        }


        if level.is_empty() {
            levels.remove(&price);
        }

        // 4) Remove from arena
        self.arena.remove(node_index);

        // 5) Remove from index
        self.order_index.remove(&id);

        true
    }

}




// #include <iostream>
// #include <vector>
// #include <string>
// #include <cmath>

// using namespace std;


// vector<int> get_ascii_code(string &msg){
//     int len=msg.length();
    
//     vector<int> ascii;
//     for(int i=0;i<len;i++){
//         int val=msg[i];
//         ascii.push_back(val);
//     }
//     return ascii;
// }
// void display(vector<int>&vec,string str){
//     cout<<str<<" : ";
//     int len=vec.size();
    
//     for(int i=0;i<len;i++){
//         cout<<vec[i]<<" ";
//     }
//     cout<<endl;
// }

// string get_binary_val(int val){
//     string str="";
//     while(val!=0){
//         int rem=val%2;
//         val=val/2;
//         char ch=rem+'0';
//         str=ch+str;
//     }
//     //cout<<str<<endl;
//     if(str.length()!=8){
//         //cout<<"yes"<<endl;
//         int len=str.length();
//         for(int i=0;i<8-len;i++){
//             str='0'+str;
//         }
//     }
//     return str;
// }

// string get_binary_format(vector<int> &vec){
//     int len=vec.size();
//     string str="";
//     for(int i=0;i<len;i++){
//         int val=vec[i];
//         string binary_val=get_binary_val(val);
//         str=str+binary_val;
//     }
//     return str;
// }

// char get_binary_to_dec(string str,int start,int end){
//     int val=0;
//     int pos=0;
//     for(int i=end;i>=start;i--){
//         if(str[i]=='1'){
//             val=val+pow(2,pos);
//         }
//         pos++;
//     }
//     if(val>9){
//         int diff=val-10;
//         return 'a'+diff;
//     }
//     return val+'0';
// }

// string get_hex_value(string str){
//     int len=str.length();
//     string str1="";
    
//     char val1=get_binary_to_dec(str,0,3);
//     char val2=get_binary_to_dec(str,4,7);
    
//     str1=str1+val1;
//     str1=str1+val2;
    
//     return str1;
// }

// vector<string> get_hex_coded_value(string str){
//     int len=str.length();
//     cout<<"hex values : ";
//     vector<string> vec;
    
//     for(int i=0;i<len;i=i+8){
//         string temp="";
//         for(int j=i;j<i+8;j++){
//             temp=temp+str[j];
//         }
//         //cout<<temp<<" ";
//         string hex_val=get_hex_value(temp);
//         cout<<hex_val<<" ";
//         vec.push_back(hex_val);
//     }
    
//     return vec;
// }

// vector<vector<string>> get_2d_hex(vector<string>&vec){
//     vector<vector<string>>vec_2d(4,vector<string>(4));
//     int pos=0;
//     for(int i=0;i<4;i++){
//         for(int j=0;j<4;j++){
//             vec_2d[j][i]=vec[pos];
//             pos++;
//         }
//     }
//     return vec_2d;
// }

// void display2d(vector<vector<string>>vec,string str){
//     cout<<endl;cout<<str<<" : "<<endl;
//     int row=vec.size();
//     int col=vec[0].size();
    
//     for(int i=0;i<row;i++){
//         for(int j=0;j<col;j++){
//             cout<<vec[i][j]<<" ";
//         }
//         cout<<endl;
//     }
//     cout<<endl;
// }

// int get_val(char ch){
//     if(ch<'a'){
//         return ch-'0';
//     }else{
//         return 10+ch-'a';
//     }
// }

// string get_binary_from_hex(char ch){
//     int num;
//     if(ch<'a'){
//         num=ch-'0';
//     }else{
//         num=ch-'a'+10;
//     }
//     return get_binary_val(num);
// }

// char XOR_digit(char a,char b){
//     if(a=='1'||b=='1'){
//         return '1';
//     }else{
//         return '0';
//     }
// }

// string XOR_8bit(string str1,string str2){
//     int size=str1.length();
    
//     string str="";
    
//     for(int i=0;i<size;i++){
        
//         char val=XOR_digit(str1[i],str2[i]);
//         str=str+val;
//     }
//     return val;
// }

// vector<string> XOR_with_R(vector<vector<stirng>>&hex2d, vector<vector<string>>&rounds,int round_number){
//     vector<string>vec;
    
//     for(int i=0;i<4;i++){
//         string str1=hex2d[i][3];
//         string str2=rounds[round_number-1][i];
        
//         string str1_bin=get_binary_from_hex(str1[0]);
//         str1_bin=str1_bin+get_binary_from_hex(str1[1]);
        
//         string str2_bin=get_binary_from_hex(str2[0]);
//         str2_bin=str2_bin+get_binary_from_hex(str2[1]);
        
        
//         string result=XOR_8bit(str1_bin,str2_bin);
//         string result_hex=get_hex_value(result);
//         vec.push_back(result_hex);
//     }
//     return vec;
// }

// string XOR(string str1,string str2){
//     string str1_bin=get_binary_from_hex(str1[0]);
//     str1_bin=str1_bin+get_binary_from_hex(str1[1]);
    
//     string str2_bin=get_binary_from_hex(str2[0]);
//     str2_bin=str2_bin+get_binary_from_hex(str2[1]);
    
    
//     string result=XOR_8bit(str1_bin,str2_bin);
//     string result_hex=get_hex_value(result);
    
//     return result_hex;
// }

// vector<vector<string>> round_operation(vector<vector<string>>&sbox, 
//     vector<vector<string>>&hex2d,
//     vector<vector<string>>&rounds,
//     int round_number){
//     //RT vector
//     string temp=hex2d[0][3];
//     vector<vector<string>> hex_new_2d(4,vector<string>(4));
//     for(int i=1;i<4;i++){
//         hex_new_2d[i-1][0]=hex2d[i][3];
//     }
//     hex_new_2d[3][0]=temp;
    
    
//     //S-box table 
//     for(int i=0;i<4;i++){
//         string val=hex_new_2d[i][0];
        
//         char ch1=val[0];
//         char ch2=val[1];
        
//         int row=get_val(ch1);
//         int col=get_val(ch2);
        
//         hex_new_2d[i][0]=sbox[row][col];
//     }
    
//     //ab karenge XOR operation R matrix ke sath 
//     vector<string> res=XOR_with_R(hex2d,rounds,1);
    
//     for(int i=0;i<4;i++){
//         hex_new_2d[i][0]=res[i];
//     }
    
//     ///g ka fn khatam
    
//     //ab karenge xor wala op
    
//     for(int i=0;i<4;i++){
//         hex_new_2d[i][0]=XOR(hex_new_2d[i][0],hex2d[i][0])
//     }
    
    
//     for(int j=1;j<4;j++){
//         for(int i=0;i<4;i++){
//             hex_new_2d[i][j]=XOR(hex_new_2d[i][j-1],hex2d[i][j]);
//         }
//     }
//     return hex_new_2d;
// }

// void encrypt(vector<vector<string>>&s_box, vector<vector<string>>&hex2d){
    
//     for(int i=1;i<=10;i++){
//         vector<vector<string>> hex_new_2d=round_operation(sbox,hex2d,rounds,i);
//     }
// }

// int main(){
//     // string msg="satishcjisboring";
    
//     // vector<int> ascii_code=get_ascii_code(msg);
//     // display(ascii_code,"ascii_code");
    
//     // string binary_rep=get_binary_format(ascii_code);
//     // cout<<"binary encoded length : "<<binary_rep.length()<<endl;
//     // cout<<"binary encoded : "<<binary_rep<<endl;
//     // vector<string>hex_values=get_hex_coded_value(binary_rep);
    
//     // vector<vector<string>>hex2d=get_2d_hex(hex_values);
    
//     // display2d(hex2d,"hex2d");
//     //cout<<get_binary_to_dec("1111",0,3);
//     //cout<<get_hex_value("11000001");
    
//     vector<vector<string>> s_box = {
//         {"63","7c","77","7b","f2","6b","6f","c5","30","01","67","2b","fe","d7","ab","76"},
//         {"ca","82","c9","7d","fa","59","47","f0","ad","d4","a2","af","9c","a4","72","c0"},
//         {"b7","fd","93","26","36","3f","f7","cc","34","a5","e5","f1","71","d8","31","15"},
//         {"04","c7","23","c3","18","96","05","9a","07","12","80","e2","eb","27","b2","75"},
//         {"09","83","2c","1a","1b","6e","5a","a0","52","3b","d6","b3","29","e3","2f","84"},
//         {"53","d1","00","ed","20","fc","b1","5b","6a","cb","be","39","4a","4c","58","cf"},
//         {"d0","ef","aa","fb","43","4d","33","85","45","f9","02","7f","50","3c","9f","a8"},
//         {"51","a3","40","8f","92","9d","38","f5","bc","b6","da","21","10","ff","f3","d2"},
//         {"cd","0c","13","ec","5f","97","44","17","c4","a7","7e","3d","64","5d","19","73"},
//         {"60","81","4f","dc","22","2a","90","88","46","ee","b8","14","de","5e","0b","db"},
//         {"e0","32","3a","0a","49","06","24","5c","c2","d3","ac","62","91","95","e4","79"},
//         {"e7","c8","37","6d","8d","d5","4e","a9","6c","56","f4","ea","65","7a","ae","08"},
//         {"ba","78","25","2e","1c","a6","b4","c6","e8","dd","74","1f","4b","bd","8b","8a"},
//         {"70","3e","b5","66","48","03","f6","0e","61","35","57","b9","86","c1","1d","9e"},
//         {"e1","f8","98","11","69","d9","8e","94","9b","1e","87","e9","ce","55","28","df"},
//         {"8c","a1","89","0d","bf","e6","42","68","41","99","2d","0f","b0","54","bb","16"}
//     };
    
//     vector<vector<string>>rounds={
//         {"01","00","00","00"},
//         {"02","00","00","00"},
//         {"04","00","00","00"},
//         {"08","00","00","00"},
//         {"10","00","00","00"},
//         {"20","00","00","00"},
//         {"40","00","00","00"},
//         {"80","00","00","00"},
//         {"1B","00","00","00"},
//         {"36","00","00","00"}
//     };
    
//     ///encrypt(s_box,hex2d);
//     //cout<<get_val('f');
// }