use arboard::Clipboard;
use std::fs::File;
use std::io::Write;
use serde::Serialize;


fn read_clipboard() -> Result<String, arboard::Error> {
    let mut clipboard = Clipboard::new()?;
    let clipboard_data = clipboard.get_text().unwrap_or_else(|_| String::from("No text in clipboard"));
    Ok(clipboard_data)
}

fn set_clipboard(text: &str) -> Result<(), arboard::Error> {
    let mut clipboard = Clipboard::new()?;
    clipboard.set_text(text.to_string())
}

fn main() {
    
    let clipboard_data = read_clipboard().expect("Failed to read clipboard");
    println!("Clipboard text was: {}", clipboard_data);

    // Create a file to write the clipboard text to
    let mut file = File::create("clipboard_text.txt").expect("Could not create file");
    file.write_all(clipboard_data.as_bytes())
        .expect("Could not write to file");


    set_clipboard("Hello, world!").expect("Failed to set clipboard text");
    //let the_string = "Hello, world!";
    //clipboard.set_text(the_string).unwrap();
    //println!("But now the clipboard text should be: \"{}\"", the_string);
}

