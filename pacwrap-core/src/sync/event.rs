pub mod download;
pub mod query;
pub mod progress;

fn whitespace(total: usize, current: usize) -> String {
    let mut whitespace = String::new();
    let difference = total-current;
  
    if difference > 0 {
        for _ in 0..difference {
            whitespace.push_str(" ");
        } 
    }

    whitespace
}
