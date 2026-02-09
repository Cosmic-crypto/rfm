# RFM (Rust File Manager)
---
## About
RFM is a CLi tool built in Rust that is open-source and is also meant to pack all of the uses of tools like: curl, wget, del all into one powerful and fast tool: RFM
---

## How to install
you can install rfm by doing either one:

### Using cargo
You can download rust by going to and following the instructions here: [Rust download](https://rust-lang.org/tools/install/)
this installs the rust toolchain, the main ones are:
* cargo (rust package manager)
* rustc (rust compiler)
* rustup (everything to do with rust installation and mode)

now run the following:
```bash
cargo new rfm # or any prefered name
cd rfm # or the name you gave the folder
# swap the src/main.rs with the one provided in this repo and hit Ctrl + S to save
cargo install --path . # install it to your user path so you can call it using rfm or the name you gave it
where rfm # to check if it's actually been installed
```

now you may, if you want, delete rust with the following command:
```bash
rustup self uninstall
```

### Downloading the compiled binary
If you go to the releases of this repo you will find a .exe you can install now you can place this .exe anywhere and call it (as long as you are in the same directory
---

## Usage
Once you have RFM installed you now have an entire file manager in your terminal, and here is how to use it:

### To install
```bash
rfm [-i/--install] <path> --url <url>
```

### To delete
```bash
rfm [-d/--delete] <path>
```

### To move
```bash
rfm [-m/--move-file] <path> --move-to <new path>
```
---

## To Improve
- there are a couple more things i have planned out, such as adding more opening and renaming
- add a progressbar for the uninstall function
- add more comments

## Feedback
This is my largest rust project, and I would definatley appreciate some feedback on my code.
