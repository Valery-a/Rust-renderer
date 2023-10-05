use math::*;
use std::vec::Vec as Vector;
use renderer::*;
use alga::general::*;
use alga::general::SupersetOf;
use std::collections::HashMap;
use matrix::*;
use typenum::{U1, U3,};
use lapack_sys::{sgeqrf_, sgesvd_};

//use matrix::*;

//uniform manifold dual contouring is a modification to dual marching cubes (hermite extension to dual marching cubes)

//dual marching cubes (modification, by Nielson, to original marching cubes)
//taken from:
//https://stackoverflow.com/questions/16638711/dual-marching-cubes-table

//original work:
//https://vis.computer.org/vis2004/DVD/vis/papers/nielson2.pdf

//256 x 16
pub fn edge_table() -> Vector< Vector< isize > >{
    vec![
            vec![-2, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![0, 8, 3, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],  
            vec![0, 1, 9, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],  
            vec![1, 3, 8, 9, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],   
            vec![1, 2, 10, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1], 
            vec![0, 8, 3, -1, 1, 2, 10, -2, -1, -1, -1, -1, -1, -1, -1, -1],    
            vec![9, 0, 2, 10, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],  
            vec![10, 2, 3, 8, 9, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],   
            vec![3, 11, 2, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1], 
            vec![0, 8, 11, 2, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],  
            vec![1, 9, 0, -1, 2, 3, 11, -2, -1, -1, -1, -1, -1, -1, -1, -1],    
            vec![1, 2, 8, 9, 11, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],   
            vec![1, 3, 10, 11, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1], 
            vec![0, 1, 8, 10, 11, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],  
            vec![0, 3, 9, 10, 11, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],  
            vec![8, 9, 10, 11, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1], 
            vec![4, 7, 8, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],  
            vec![0, 3, 4, 7, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],   
            vec![0, 1, 9, -1, 8, 4, 7, -2, -1, -1, -1, -1, -1, -1, -1, -1],     
            vec![1, 3, 4, 7, 9, -2, -1, -1, 1, -1, -1, -1, -1, -1, -1, -1],              
            vec![1, 2, 10, -1, 8, 4, 7, -2, -1, -1, -1, -1, -1, -1, -1, -1],    
            vec![1, 2, 10, -1, 0, 3, 7, 4, -2, -1, -1, -1, -1, -1, -1, -1],     
            vec![0, 2, 10, 9, -1, 8, 7, 4, -2, -1, -1, -1, -1, -1, -1, -1],     
            vec![2, 3, 4, 7, 9, 10, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1],    
            vec![8, 4, 7, -1, 3, 11, 2, -2, -1, -1, -1, -1, -1, -1, -1, -1],    
            vec![0, 2, 4, 7, 11, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],   
            vec![9, 0, 1, -1, 8, 4, 7, -1, 2, 3, 11, -2, -1, -1, -1, -1],       
            vec![1, 2, 4, 7, 9, 11, -2, -1, -1, -1, -1, -1-1, -1, -1, -1, -1],              
            vec![3, 11, 10, 1, -1, 8, 7, 4, -2, -1, -1, -1, -1, -1, -1, -1],    
            vec![0, 1, 4, 7, 10, 11, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1],   
            vec![4, 7, 8, -1, 0, 3, 11, 10, 9, -2, -1, -1, -1, -1, -1, -1],     
            vec![4, 7, 9, 10, 11, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],  
            vec![9, 5, 4, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],  
            vec![9, 5, 4, -1, 0, 8, 3, -2, -1, -1, -1, -1, -1, -1, -1, -1],     
            vec![0, 5, 4, 1, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],   
            vec![1, 3, 4, 5, 8, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],    
            vec![1, 2, 10, -1, 9, 5, 4, -2, -1, -1, -1, -1, -1, -1, -1, -1],    
            vec![3, 0, 8, -1, 1, 2, 10, -1, 4, 9, 5, -2, -1, -1, -1, -1],       
            vec![0, 2, 4, 5, 10, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],   
            vec![2, 3, 4, 5, 8, 10, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1],    
            vec![9, 5, 4, -1, 2, 3, 11, -2, -1, -1, -1, -1, -1, -1, -1, -1],    
            vec![9, 4, 5, -1, 0, 2, 11, 8, -2, -1, -1, -1, -1, -1, -1, -1],     
            vec![3, 11, 2, -1, 0, 4, 5, 1, -2, -1, -1, -1, -1, -1, -1, -1],     
            vec![1, 2, 4, 5, 8, 11, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1],    
            vec![3, 11, 10, 1, -1, 9, 4, 5, -2, -1, -1, -1, -1, -1, -1, -1],    
            vec![4, 9, 5, -1, 0, 1, 8, 10, 11, -2, -1, -1, -1, -1, -1, -1],     
            vec![0, 3, 4, 5, 10, 11, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1],   
            vec![5, 4, 8, 10, 11, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],  
            vec![5, 7, 8, 9, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],   
            vec![0, 3, 5, 7, 9, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],    
            vec![0, 1, 5, 7, 8, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],    
            vec![1, 5, 3, 7, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],   
            vec![1, 2, 10, -1, 8, 7, 5, 9, -2, -1, -1, -1, -1, -1, -1, -1],     
            vec![1, 2, 10, -1, 0, 3, 7, 5, 9, -2, -1, -1, -1, -1, -1, -1],      
            vec![0, 2, 5, 7, 8, 10, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1],    
            vec![2, 3, 5, 7, 10, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],   
            vec![2, 3, 11, -1, 5, 7, 8, 9, -2, -1, -1, -1, -1, -1, -1, -1],            
            vec![0, 2, 5, 7, 9, 11, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1],    
            vec![2, 3, 11, -1, 0, 1, 5, 7, 8, -2, -1, -1, -1, -1, -1, -1],      
            vec![1, 2, 5, 7, 11, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],   
            vec![3, 11, 10, 1, -1, 8, 7, 5, 9, -2, -1, -1, -1, -1, -1, -1],     
            vec![0, 1, 5, 7, 9, 10, 11, -2, -1, -1, -1, -1, -1, -1, -1, -1],    
            vec![0, 3, 5, 7, 8, 10, 11, -2, -1, -1, -1, -1, -1, -1, -1, -1],    
            vec![5, 7, 10, 11, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![5, 6, 10, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![0, 3, 8, -1, 10, 5, 6, -2, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![0, 1, 9, -1, 10, 5, 6, -2, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![10, 5, 6, -1, 3, 8, 9, 1, -2, -1, -1, -1, -1, -1, -1, -1],
            vec![1, 2, 5, 6, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![0, 3, 8, -1, 1, 2, 6, 5, -2, -1, -1, -1, -1, -1, -1, -1],
            vec![0, 2, 5, 6, 9, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![2, 3, 5, 6, 8, 9, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![3, 11, 2, -1, 10, 6, 5, -2, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![10, 5, 6, -1, 0, 8, 2, 11, -2, -1, -1, -1, -1, -1, -1, -1],
            vec![3, 11, 2, -1, 0, 1, 9, -1, 10, 5, 6, -2, -1, -1, -1, -1],
            vec![10, 5, 6, -1, 11, 2, 8, 9, 1, -2, -1, -1, -1, -1, -1, -1],
            vec![1, 3, 5, 6, 11, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![0, 1, 5, 6, 8, 11, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![0, 3, 5, 6, 9, 11, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![5, 6, 8, 9, 11, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![8, 7, 4, -1, 5, 6, 10, -2, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![5, 6, 10, -1, 0, 3, 4, 7, -2, -1, -1, -1, -1, -1, -1, -1],
            vec![10, 5, 6, -1, 1, 9, 0, -1, 8, 7, 4, -2, -1, -1, -1, -1],
            vec![10, 5, 6, -1, 7, 4, 9, 1, 3, -2, -1, -1, -1, -1, -1, -1],
            vec![8, 7, 4, -1, 1, 2, 6, 5, -2, -1, -1, -1, -1, -1, -1, -1],
            vec![0, 3, 7, 4, -1, 1, 2, 6, 5, -2, -1, -1, -1, -1, -1, -1],
            vec![8, 7, 4, -1, 0, 9, 2, 5, 6, -2, -1, -1, -1, -1, -1, -1],
            vec![2, 3, 4, 5, 6, 7, 9, -2, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![3, 11, 2, -1, 5, 6, 10, -1, 8, 7, 4, -2, -1, -1, -1, -1],
            vec![10, 5, 6, -1, 0, 2, 11, 7, 4, -2, -1, -1, -1, -1, -1, -1],
            vec![3, 11, 2, -1, 0, 1, 9, -1, 10, 5, 6, -1, 8, 7, 4, -2],
            vec![10, 5, 6, -1, 7, 4, 11, 2, 1, 9, -2, -1, -1, -1, -1, -1],
            vec![8, 7, 4, -1, 3, 11, 6, 5, 1, -2, -1, -1, -1, -1, -1, -1],
            vec![0, 1, 4, 5, 6, 7, 11, -2, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![8, 7, 4, -1, 6, 5, 9, 0, 11, 3, -2, -1, -1, -1, -1, -1],
            vec![4, 5, 6, 7, 9, 11, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1],             
            vec![4, 6, 9, 10, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![0, 3, 8, -1, 9, 10, 6, 4, -2, -1, -1, -1, -1, -1, -1, -1],
            vec![0, 1, 4, 6, 10, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![1, 3, 4, 6, 8, 10, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![1, 2, 4, 6, 9, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![0, 3, 8, -1, 1, 2, 4, 6, 9, -2, -1, -1, -1, -1, -1, -1],
            vec![0, 2, 4, 6, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![2, 3, 4, 6, 8, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![11, 2, 3, -1, 9, 4, 10, 6, -2, -1, -1, -1, -1, -1, -1, -1],
            vec![0, 2, 11, 8, -1, 9, 4, 6, 10, -2, -1, -1, -1, -1, -1, -1],
            vec![2, 3, 11, -1, 0, 1, 4, 6, 10, -2, -1, -1, -1, -1, -1, -1],
            vec![1, 2, 4, 6, 8, 10, 11, -2, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![1, 3, 4, 6, 9, 11, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![0, 1, 4, 6, 8, 9, 11, -2, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![0, 3, 4, 6, 11, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![4, 6, 8, 11, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![6, 7, 8, 9, 10, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![0, 3, 6, 7, 9, 10, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![0, 1, 6, 7, 8, 10, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![1, 3, 6, 7, 10, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![1, 2, 6, 7, 8, 9, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![0, 1, 2, 3, 6, 7, 9, -2, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![0, 2, 6, 7, 8, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![2, 3, 6, 7, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![3, 11, 2, -1, 10, 6, 9, 7, 8, -2, -1, -1, -1, -1, -1, -1],
            vec![0, 2, 6, 7, 9, 10, 11, -2, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![3, 11, 2, -1, 8, 7, 0, 1, 10, 6, -2, -1, -1, -1, -1, -1],
            vec![1, 2, 6, 7, 10, 11, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![1, 3, 6, 7, 8, 9, 11, -2, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![0, 1, 6, 7, 9, 11, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![0, 3, 6, 7, 8, 11, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![6, 7, 11, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![11, 7, 6, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![11, 7, 6, -1, 0, 3, 8, -2, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![11, 7, 6, -1, 0, 9, 1, -2, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![11, 7, 6, -1, 1, 3, 8, 9, -2, -1, -1, -1, -1, -1, -1, -1],
            vec![11, 7, 6, -1, 1, 2, 10, -2, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![11, 7, 6, -1, 1, 2, 10, -1, 0, 3, 8, -2, -1, -1, -1, -1],
            vec![11, 7, 6, -1, 0, 9, 10, 2, -2, -1, -1, -1, -1, -1, -1, -1],
            vec![11, 7, 6, -1, 2, 3, 8, 9, 10, -2, -1, -1, -1, -1, -1, -1],
            vec![2, 3, 6, 7, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![0, 2, 6, 7, 8, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![0, 1, 9, -1, 3, 2, 6, 7, -2, -1, -1, -1, -1, -1, -1, -1],
            vec![1, 2, 6, 7, 8, 9, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![1, 3, 6, 7, 10, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![0, 1, 6, 7, 8, 10, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![0, 3, 6, 7, 9, 10, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![6, 7, 8, 9, 10, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![4, 6, 8, 11, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![0, 3, 4, 6, 11, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![0, 1, 9, -1, 8, 4, 11, 6, -2, -1, -1, -1, -1, -1, -1, -1],
            vec![1, 3, 4, 6, 9, 11, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![1, 2, 10, -1, 8, 4, 6, 11, -2, -1, -1, -1, -1, -1, -1, -1],
            vec![1, 2, 10, -1, 0, 3, 11, 6, 4, -2, -1, -1, -1, -1, -1, -1],
            vec![0, 9, 10, 2, -1, 8, 4, 11, 6, -2, -1, -1, -1, -1, -1, -1],
            vec![2, 3, 4, 6, 9, 10, 11, -2, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![2, 3, 4, 6, 8, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![0, 2, 4, 6, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![0, 1, 9, -1, 2, 3, 8, 4, 6, -2, -1, -1, -1, -1, -1, -1],
            vec![1, 2, 4, 6, 9, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![1, 3, 4, 6, 8, 10, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![0, 1, 4, 6, 10, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![0, 3, 4, 6, 8, 9, 10, -2, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![4, 6, 9, 10, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![6, 7, 11, -1, 4, 5, 9, -2, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![6, 7, 11, -1, 4, 5, 9, -1, 0, 3, 8, -2, -1, -1, -1, -1],
            vec![11, 7, 6, -1, 1, 0, 5, 4, -2, -1, -1, -1, -1, -1, -1, -1],
            vec![11, 7, 6, -1, 8, 3, 1, 5, 4, -2, -1, -1, -1, -1, -1, -1],
            vec![11, 7, 6, -1, 4, 5, 9, -1, 1, 2, 10, -2, -1, -1, -1, -1],
            vec![11, 7, 6, -1, 4, 5, 9, -1, 1, 2, 10, -1, 0, 3, 8, -2],
            vec![11, 7, 6, -1, 0, 2, 10, 5, 4, -2, -1, -1, -1, -1, -1, -1],
            vec![11, 7, 6, -1, 8, 3, 2, 10, 5, 4, -2, -1, -1, -1, -1, -1],
            vec![4, 5, 9, -1, 3, 2, 6, 7, -2, -1, -1, -1, -1, -1, -1, -1],
            vec![4, 5, 9, -1, 2, 0, 8, 7, 6, -2, -1, -1, -1, -1, -1, -1],
            vec![3, 2, 6, 7, -1, 0, 1, 5, 4, -2, -1, -1, -1, -1, -1, -1],
            vec![1, 2, 4, 5, 6, 7, 8, -2, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![9, 4, 5, -1, 1, 10, 6, 7, 3, -2, -1, -1, -1, -1, -1, -1],
            vec![9, 4, 5, -1, 6, 10, 1, 0, 8, 7, -2, -1, -1, -1, -1, -1],
            vec![0, 3, 4, 5, 6, 7, 10, -2, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![4, 5, 6, 7, 8, 10, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![5, 6, 8, 9, 11, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![0, 3, 5, 6, 9, 11, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![0, 1, 5, 6, 8, 11, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![1, 3, 5, 6, 11, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![1, 2, 10, -1, 9, 5, 6, 11, 8, -2, -1, -1, -1, -1, -1, -1],
            vec![1, 2, 10, -1, 9, 0, 3, 11, 6, 5, -2, -1, -1, -1, -1, -1],
            vec![0, 2, 5, 6, 8, 10, 11, -2, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![2, 3, 5, 6, 10, 11, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![2, 3, 5, 6, 8, 9, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![0, 2, 5, 6, 9, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![0, 1, 2, 3, 5, 6, 8, -2, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![1, 2, 5, 6, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![1, 3, 5, 6, 8, 9, 10, -2, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![0, 1, 5, 6, 9, 10, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![0, 3, 5, 6, 8, 10, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![5, 6, 10, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![5, 7, 10, 11, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![5, 7, 10, 11, -1, 0, 3, 8, -2, -1, -1, -1, -1, -1, -1, -1],
            vec![5, 7, 10, 11, -1, 0, 1, 9, -2, -1, -1, -1, -1, -1, -1, -1],
            vec![5, 7, 10, 11, -1, 3, 8, 9, 1, -2, -1, -1, -1, -1, -1, -1],
            vec![1, 2, 5, 7, 11, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![1, 2, 5, 7, 11, -1, 0, 3, 8, -2, -1, -1, -1, -1, -1, -1],
            vec![0, 2, 5, 7, 9, 11, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![2, 3, 5, 7, 8, 9, 11, -2, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![2, 3, 5, 7, 10, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![0, 2, 5, 7, 8, 10, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![0, 1, 9, -1, 7, 3, 2, 10, 5, -2, -1, -1, -1, -1, -1, -1],
            vec![1, 2, 5, 7, 8, 9, 10, -2, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![1, 3, 5, 7, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![0, 1, 5, 7, 8, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![0, 3, 5, 7, 9, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![5, 7, 8, 9, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![4, 5, 8, 10, 11, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![0, 3, 4, 5, 10, 11, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![0, 1, 9, -1, 8, 11, 10, 4, 5, -2, -1, -1, -1, -1, -1, -1],
            vec![1, 3, 4, 5, 9, 10, 11, -2, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![1, 2, 4, 5, 8, 11, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![0, 1, 2, 3, 4, 5, 11, -2, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![0, 2, 4, 5, 8, 9, 11, -2, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![2, 3, 4, 5, 9, 11, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![2, 3, 4, 5, 8, 10, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![0, 2, 4, 5, 10, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![0, 1, 9, -1, 8, 3, 2, 10, 5, 4, -2, -1, -1, -1, -1, -1],
            vec![1, 2, 4, 5, 9, 10, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![1, 3, 4, 5, 8, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![0, 1, 4, 5, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![0, 3, 4, 5, 8, 9, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![4, 5, 9, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![4, 7, 9, 10, 11, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![0, 3, 8, -1, 10, 9, 4, 7, 11, -2, -1, -1, -1, -1, -1, -1],
            vec![0, 1, 4, 7, 10, 11, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![1, 3, 4, 7, 8, 10, 11, -2, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![1, 2, 4, 7, 9, 11, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![1, 2, 4, 7, 9, 11, -1, 0, 3, 8, -2, -1, -1, -1, -1, -1],
            vec![0, 2, 4, 7, 11, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![2, 3, 4, 7, 8, 11, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![2, 3, 4, 7, 9, 10, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![0, 2, 4, 7, 8, 9, 10, -2, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![0, 1, 2, 3, 4, 7, 10, -2, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![1, 2, 4, 7, 8, 10, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![1, 3, 4, 7, 9, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![0, 1, 4, 7, 8, 9, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![0, 3, 4, 7, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![4, 7, 8, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![8, 9, 10, 11, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![0, 3, 9, 10, 11, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![0, 1, 8, 10, 11, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![1, 3, 10, 11, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![1, 2, 8, 9, 11, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![0, 1, 2, 3, 9, 11, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![0, 2, 8, 11, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![2, 3, 11, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![2, 3, 8, 9, 10, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![0, 2, 9, 10, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![0, 1, 2, 3, 8, 10, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![1, 2, 10, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![1, 3, 8, 9, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![0, 1, 9, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![0, 3, 8, -2, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1],
            vec![-2, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1]

    ]
}

pub fn corner_points() -> Vector< Vec3<f32> >{
    vec![
        Vec3::new(0.0,0.0,0.0),
        Vec3::new(1.0,0.0,0.0),
        Vec3::new(1.0,0.0,1.0),  //clockwise starting from zero y min
        Vec3::new(0.0,0.0,1.0),

        Vec3::new(0.0,1.0,0.0),
        Vec3::new(1.0,1.0,0.0), //y max
        Vec3::new(1.0,1.0,1.0),
        Vec3::new(0.0,1.0,1.0)

    ]
}


pub fn edge_pairs() -> Vector< Vec2<usize> >{
    vec![
        Vec2::new(0,1),
        Vec2::new(1,2),
        Vec2::new(3,2),
        Vec2::new(0,3),

        Vec2::new(4,5),
        Vec2::new(5,6), //5
        Vec2::new(7,6), //6
        Vec2::new(4,7),

        Vec2::new(4,0),
        Vec2::new(1,5),
        Vec2::new(2,6), //10
        Vec2::new(3,7)
    ]
}

#[derive(Clone, Debug)]
pub struct Cell<T : Real>{
    pub densities : [T;8], //sampled densities at each corner, `i` element corresponds to `i` corner vector of `corner_points` fn.
    pub hermite_data : HashMap<usize, Plane<T>>, //binding between index of a particular edge of the cell and intersection point and normal for that edge
    pub config : usize, //identifier of this particular cell configuration, used in dual marching cubes table
}

#[derive(Clone, Debug)]
pub struct HermiteGrid<T : Real>{
    pub a : T,//length of one edge of a cubic cell
    pub size : usize, //number of cells along each axis
    pub cells : Vector<Option<Cell<T>>>, // length is size^3
    //after filling the grid each cell must be initialized (set to Some)
}


impl<T : Real + SupersetOf<f32>> HermiteGrid<T>{


    pub fn new(a : T, size : usize) -> HermiteGrid<T>{
        let cells = vec![None;(size + 1) * (size + 1) * (size + 1)];

        HermiteGrid{a,size,cells}
    }
    
    //cell is assumed to be initialized
    pub fn get(&self, x : usize, y : usize, z : usize) -> &Cell<T>{
        self.cells[z * self.size * self.size + y * self.size + x].as_ref().unwrap()
    }

    pub fn set(&mut self, x : usize, y : usize, z : usize, value : Cell<T>){
        let v = self.size;
        self.cells[z * v * v + y * v + x] = Some(value);
    }



    //bounding box of the cell
    pub fn cube(&self, x : usize, y : usize, z : usize, offset : Vec3<T>) -> Cube<T>{
        Cube {center : offset + Vec3::new(T::from_subset(&(x as f32 + 0.5)) * self.a, T::from_subset(&(y as f32 + 0.5)) * self.a, T::from_subset(&(z as f32 + 0.5)) * self.a), extent: self.a / T::from_subset(&2.0)}
    }
}


//it is assumed that surface is smooth in the area along the line and density at the ends of the line have different signs
//TODO handle multiple intersections per edge ???
fn sample_surface_intersection(line : Line3<f32>, n : usize, f : impl DenFn3<f32>) -> Vec3<f32>{
    let ext = line.end - line.start;
    let norm = ext.norm();
    let dir = ext * (1.0 / norm);

    //let mut best_abs = std::f32::MAX;
    //let mut best_point : Option<Vec3<f32>> = None;

    let mut center = line.start + ext * 0.5;
    let mut cur_ext = norm * 0.25;

    for _ in 0..n {
        let point1 = center - dir * cur_ext;
        let point2 = center + dir * cur_ext;
        let den1 = f(point1).abs();
        let den2 = f(point2).abs();

        if den1 <= den2 {
            center = point1;
        }else{
            center = point2;
        }
        cur_ext *= 0.5;
    }

    center
}

pub fn sample_normal(point : Vec3<f32>, eps : f32, f : impl DenFn3<f32>) -> Vec3<f32>{
    Vec3::new( f(Vec3::new(point.x + eps, point.y, point.z)) - f(Vec3::new(point.x, point.y, point.z)),
                  f(Vec3::new(point.x, point.y + eps, point.z)) - f(Vec3::new(point.x, point.y, point.z)),
                  f(Vec3::new(point.x, point.y, point.z + eps)) - f(Vec3::new(point.x, point.y, point.z)) ).normalize()
}

//outer list corresponds to each vertex to be placed inside the cell
//inner list binds edges according to the EMCT to that vertex 
pub fn which_edges_are_signed(table : &Vector< Vector<isize> >, config : usize) -> Vector<Vector<usize>>{
    let entry = &table[config];
    if entry[0] == -2 {return Vector::with_capacity(0)}
    let mut result = Vector::new();
    let mut cur_vertex = Vector::new();
    for i in 0..entry.len(){ //entry.len() is always 16
        let k = entry[i];
        if k >= 0 {cur_vertex.push(k as usize)}
        else if k == -2 {result.push(cur_vertex);return result}
        else { //k == -1
            result.push(cur_vertex);
            cur_vertex = Vector::new();
        }
    }

    result

}

fn find_minimizer_lapack(_bounds : Cube<f32>, planes : &Vector<Plane<f32>>, mass_point : Vec3<f32>) -> Vec3<f32> {
    //let mut mat : Mat<f32, U4, U6> = Mat::empty();
    let mut mat = [0f32; 24];//column-major
    let rows = planes.len();
    let columns = 4;
    //println!("planes count = {}", planes.len()); //6 planes is possible
    for i in 0..usize::min(planes.len(), 6){
        mat[0 * rows + i] = planes[i].normal.x;
        mat[1 * rows + i] = planes[i].normal.y;
        mat[2 * rows + i] = planes[i].normal.z;
        mat[3 * rows + i] = planes[i].normal.dot(planes[i].point - mass_point);
    }


    let mut tau = [0f32; 4];
    unsafe{
        let m = rows as i32;
        let n = columns as i32;
        let lda = m as i32;
        let lwork = n as i32;
        let mut work = vec![0f32; lwork as usize];
        let mut info = 0i32;

        sgeqrf_( & m, & n,  mat.as_mut_ptr(), & lda, tau.as_mut_ptr(), work.as_mut_ptr(), &lwork, &mut info);
    }


    let mut mat_a = [0f32; 9]; //column-major
    mat_a[0] = mat[0 * rows + 0];
    mat_a[4] = mat[1 * rows + 1];
    mat_a[6] = mat[2 * rows + 0];
    mat_a[7] = mat[2 * rows + 1];
    mat_a[8] = mat[2 * rows + 2];


    let b = vec3![mat[3 * rows + 0], mat[3 * rows + 1], mat[3 * rows + 2]];
    if planes.len() < 4{

    }

    let mut eigenval = Vec3::empty();
    let mut u = Mat3::empty();
    let mut ut = Mat3::empty();
    let mut cache = [0f32; 30];
    unsafe{
        let m = 3;
        let n = 3;
        let lda = 3;
        let ldu = 3;
        let ldvt = 3;
        let lwork = 30;
        let mut info = 0;

        let mut t = b'A' as i8;

        sgesvd_(&mut t, &mut t, &m, &n, mat_a.as_mut_ptr(), &lda, eigenval.as_mut_slice().as_mut_ptr(), u.as_mut_slice().as_mut_ptr(), &ldu, ut.as_mut_slice().as_mut_ptr(), &ldvt, cache.as_mut_ptr(), &lwork, &mut info);
    }



    let truncate_eps = 0.1;
    let eigenval_mapped = Mat::<f32, U3, U1>{ar : eigenval.ar.map(|v| if v.abs() > truncate_eps {1.0/v} else {0.0})};
    let mut mat_diag : Mat3<f32> = Mat::empty();
    mat_diag[(0, 0)] = eigenval_mapped[0];
    mat_diag[(1, 1)] = eigenval_mapped[1];
    mat_diag[(2, 2)] = eigenval_mapped[2];

    let mat_inverse = ut * mat_diag * u;

    let minimizer = mat_inverse * b + mass_point;

    minimizer
}

//constructs grid: calculates hermite data and configuration for each cell
//TODO generating triangles right in this function would benefit performance (no extra looping through cells)
pub fn construct_grid(f : impl DenFn3<f32>, offset : Vec3<f32>, a : f32, size : usize, accuracy : usize, render_tr_light : &mut RendererVertFragDef<()>, render_debug_lines : &mut RendererVertFragDef<()>, triangles_for_rt : &mut Vector<Triangle3<f32>>) -> HermiteGrid<f32>{
    let corners = corner_points();
    let edge_pairs = edge_pairs();
    let edge_table = edge_table();

    //bindings between edge and vertex for each cell
    let mut cache : Vector< Option< HashMap<usize, Vec3<f32>  > > > = vec![None;size * size * size];

    let mut load_cell = |grid : &mut HermiteGrid<f32>, x : usize, y : usize, z : usize, cache : &mut Vector<Option<HashMap<usize,Vec3<f32>>>>|{
        let cell_min = offset + Vec3::new(x as f32 * a, y as f32 * a, z as f32 * a);
        let bounds = grid.cube(x,y,z,offset);
        let mut densities = [0.0;8];
        let mut config = 0;
        for i in 0..8{
            let p = cell_min + corners[i] * a;
            densities[i] = f(p);
            if densities[i] < 0.0{
                config |= 1 << i;
            }
        }

        let vertices = which_edges_are_signed(&edge_table, config);

        let mut hermite_data = HashMap::new();

        let mut cached_cell = HashMap::new();

        if vertices.len() == 1 { //render cells that contain more than 1 vertex
            add_cube_bounds_pos_color(render_debug_lines, bounds.clone(), Vec3::new(0.0, 1.0, 0.0));
        }
        if vertices.len() > 1 { //render cells that contain more than 1 vertex
            add_cube_bounds_pos_color(render_debug_lines, bounds.clone(), Vec3::new(1.0, 0.0, 0.0));
        }

        for vertex in vertices{


            let mut cur_planes = Vector::with_capacity(vertex.len());
            let mut mass_point = Vec3::empty();
            

            for edge_id in &vertex{
                let pair = edge_pairs[edge_id.clone()];
                let v1 = corners[pair.x];
                let v2 = corners[pair.y];

                let edge = Line3{start : cell_min + v1 * a, end : cell_min + v2 * a};

                let intersection = sample_surface_intersection(edge, ((accuracy as f32).log2() as usize) + 1, f);
                
                let normal = sample_normal(intersection, a / 1024.0, f);

                mass_point += intersection;

                /*if normal.x.is_nan() || normal.y.is_nan() || normal.z.is_nan() {
                    //println!("nan in normal !");
                    //println!("intersection {}", intersection);
                    //exit(1);
                    //add_cube_bounds_pos_color(render_debug_lines, bounds.clone(), Vec3::new(1.0, 0.0, 0.0));
                    normal = Vec3::empty();
                    let plane = Plane{point : intersection, normal};
                    hermite_data.insert(edge_id.clone(), plane);
                    continue; // do not push zero normal to the planes
                    //weighted normals for such cases for proper lighting ?
                }*/
                
                // corners
                add_cube_bounds_pos_color(render_debug_lines, Cube{center : intersection, extent : bounds.extent / 16.0}, Vec3::new(1.0, 1.0, 1.0));

                let plane = Plane{point : intersection, normal};
                hermite_data.insert(edge_id.clone(), plane);
                cur_planes.push(plane); //for current vertex QEF processing
            }
            mass_point *= 1.0 / cur_planes.len() as f32;
           

            let minimizer = find_minimizer_lapack(bounds, &cur_planes, mass_point);
            // corners
            add_cube_bounds_pos_color(render_debug_lines, Cube{center : minimizer, extent : bounds.extent / 16.0}, Vec3::new(1.0, 1.0, 0.0));


            for edge_id in &vertex { 
                cached_cell.insert(edge_id.clone(), minimizer);//duplicates are not possible
            }

        }

        let t = z * size * size + y * size + x;
        cache[t] = Some(cached_cell);

        let cell = Cell{densities, hermite_data, config};

        grid.set(x, y, z, cell);

    };

    let mut load_cell_cached = |grid : &mut HermiteGrid<f32>, x : usize, y : usize, z : usize, cache : &mut Vector<Option<HashMap<usize,Vec3<f32>>>>|{
        let t = z * size * size + y * size + x;

        let mut load = false;
        {
            let cached = cache[t].as_ref();
            match cached{
                None => load = true,
                Some(_) => (),
            }
        }

        let cached = {
            if load{
                load_cell(grid, x, y, z, cache);
            };
            cache[t].as_ref().unwrap()
        };

        cached.clone() //TODO cloning is bad here
        
    };

    let mut grid = HermiteGrid::new(a, size);
    for y in 0..size-1{
        for z in 0..size-1{
            for x in 0..size-1{

                let cell = load_cell_cached(&mut grid, x,y,z, &mut cache);
                for (edge_id, minimizer) in &cell{
                    let t = minimizer.clone();
                    match edge_id.clone(){ //TODO add triangle vertex only once, use indexing + culling (decide direction by normal)
                        5 => {
                            let r = load_cell_cached(&mut grid, x+1,y,z, &mut cache).get(&7).unwrap().clone();
                            let ru = load_cell_cached(&mut grid, x+1,y+1,z, &mut cache).get(&3).unwrap().clone();
                            let u = load_cell_cached(&mut grid, x,y+1,z, &mut cache).get(&1).unwrap().clone();
                            let normal = grid.get(x,y,z).hermite_data.get(&5).unwrap().normal;
                            add_triangle_pos_color_normal(render_tr_light, Triangle3{p1 : t, p2 : r, p3 : ru}, Vec3::new(1.0, 0.0, 1.0), normal);
                            add_triangle_pos_color_normal(render_tr_light, Triangle3{p1 : t, p2 : ru, p3 : u}, Vec3::new(1.0, 0.0, 1.0), normal);
                            triangles_for_rt.push(Triangle3{p1 : t, p2 : r, p3 : ru});
                            triangles_for_rt.push(Triangle3{p1 : t, p2 : ru, p3 : u});
                        },
                        6 => {
                            let f = load_cell_cached(&mut grid, x,y,z+1, &mut cache).get(&4).unwrap().clone();
                            let fu_ = load_cell_cached(&mut grid, x,y+1,z+1, &mut cache);
                            // let config = grid.get(x,y+1,z+1).config;
                            // println!("vertex count {:?}, edges: {:?}, map : {:?}", vertex_num_table()[config], edge_table[config], &fu_);
                            let fu = fu_.get(&0).unwrap().clone();
                            let u = load_cell_cached(&mut grid, x,y+1,z, &mut cache).get(&2).unwrap().clone();
                            let normal = grid.get(x,y,z).hermite_data.get(&6).unwrap().normal;
                            add_triangle_pos_color_normal(render_tr_light, Triangle3{p1 : t, p2 : f, p3 : fu}, Vec3::new(1.0, 0.0, 1.0), normal);
                            add_triangle_pos_color_normal(render_tr_light, Triangle3{p1 : t, p2 : fu, p3 : u}, Vec3::new(1.0, 0.0, 1.0), normal);
                            triangles_for_rt.push(Triangle3{p1 : t, p2 : f, p3 : fu});
                            triangles_for_rt.push(Triangle3{p1 : t, p2 : fu, p3 : u});
                        },
                        10 => {
                            let r_ = load_cell_cached(&mut grid, x+1,y,z, &mut cache);
                            //let config = grid.get(x+1,y,z).config;
                            //let config_this = grid.get(x,y,z).config;
                            //println!("this {:?}, errored {:?}", &edge_table[config_this], &edge_table[config]);
                            let r = r_.get(&11).unwrap().clone();
                            let rf = load_cell_cached(&mut grid, x+1,y,z+1, &mut cache).get(&8).unwrap().clone();
                            let f = load_cell_cached(&mut grid, x,y,z+1, &mut cache).get(&9).unwrap().clone();

                            let normal = grid.get(x,y,z).hermite_data.get(&10).unwrap().normal;

                            add_triangle_pos_color_normal(render_tr_light, Triangle3{p1 : t, p2 : rf, p3 : r}, Vec3::new(1.0, 0.0, 1.0), normal);
                            add_triangle_pos_color_normal(render_tr_light, Triangle3{p1 : t, p2 : f, p3 : rf}, Vec3::new(1.0, 0.0, 1.0), normal);
                            triangles_for_rt.push(Triangle3{p1 : t, p2 : rf, p3 : r});
                            triangles_for_rt.push(Triangle3{p1 : t, p2 : f, p3 : rf});
                        },
                        _ => ()
                    }
                }
                
            }
        }
    }

    grid
}