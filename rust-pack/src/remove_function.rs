fn pick_char(input: &mut &str) -> char {
    let mut iter = input.chars();
    let r = iter.next().unwrap();
    *input = iter.as_str();
    r
}

fn eat_until(input: &mut &str, output: &mut String, goal: char) {
    let mut paran = if goal == 'x' { 2 } else { 0 };
    while input.len() > 0 {
        let c = input.chars().next().unwrap();
        if paran > 0 {
            if c == ')' || c == ']' || c == '}' {
                paran -= 1;
            }
        } else {
            if c == goal || goal == 'x' {
                return;
            }
            if c == '(' || c == '[' || c == '{' {
                paran += 1;
            }
        }
        pick_char(input);
        output.push(c);
    }
}

pub fn remove_function_body(mut input: &str) -> String {
    let mut output = String::new();
    let mut char_seened = 'x';
    while input.len() > 0 {
        if char_seened.is_whitespace() {
            if let Some(remain) = input.strip_prefix("fn ") {
                output.push_str("fn ");
                input = remain;
                eat_until(&mut input, &mut output, '{');
                output.push_str("{ loop {} }");
                eat_until(&mut input, &mut String::new(), 'x');
            }
        }
        if input.starts_with("//") {
            let (comment, remain) = input.split_once("\n").unwrap();
            output.push_str(comment);
            input = remain;
        }
        char_seened = pick_char(&mut input);
        output.push(char_seened);
    }
    output
}
