use std::ops::Range;

unsafe fn gav(x: i32, y: i32) -> i64 {
    (x - y) * (x + y)
}

#[derive(Debug)]
struct Gen<G, 'a> {
    g: &'a G
}

impl<G, 'a> Gen<G, 'a> {
    /// Create a new `Gen`
    /// ```
    /// let mut gen = Gen::new(&mut something);
    /// ```
    fn new(g: &mut G) -> Self {
        Gen { g }
    }

    fn do(&mut self) -> () { }
}

fn main() {
    let num = 5;
    let a = vec![1, 2, 3];
    let b = Some(2);
    let c = None;
    let d = Range { start: 1, end: num };
    let e = 1..num;
    let mut f = "sssss".to_string();
    let x = &mut f;
    for a in d {
        for b in e {
            let c = unsafe { gav(gav(a, b), a) };
            assert_eq!(gav(a, b), a * a - b * b);
        }
    }

    let mut gen = Gen::new(&mut f);
    let f = d
        .reduce(|a, b| {
            gen.do();
            println!("value: {}", a);
            a * b
        })
        .unwrap();
}
