# SqFrame
SqFrame is an open-source tool written in Rust, that facilitates the creation of a square frame around an image, with a blurred background.
For example, [![image.png](https://i.postimg.cc/W1RL7c9k/image.png)](https://postimg.cc/hf05S6RD) becomes [![image.png](https://i.postimg.cc/7Y2jhQxY/image.png)](https://postimg.cc/vcG0Shgp) (notice that this one is a square, with a blurred background).
## Motivation
A few days ago (around November 5th, 2023), I realized something. I am a huge fan of the game of chess, and play games every day, about which I [post on Instagram](https://instagram.com/puissant.patzer) (check out my profile lol). Instagram allows for multiple slides (images) in posts, but all slides of a post have to be in the same aspect ratio. I always choose 1:1 (a square), and for this I find myself sending screenshots of my games over from my laptop to my phone, making a square frame with [InShot](https://play.google.com/store/apps/details?id=com.camerasideas.instashot), and sending the edited image back to my laptop to post on [instagram.com](https://instagram.com). This process gets boring when you have to do it every day, and so, like a wise programmer once said, I decided to work smarter and not harder. I decided to automate this process with a single program. Now, of course, this would've been much MUCH easier to write in Python (my first language, and the language I have the most experience with), but I was learning Rust and decided to make this my second project following my [word guessing game](https://github.com/Python3-8/word_guessing_game).

# Install
Navigate to the [latest release](https://github.com/Python3-8/sqframe/releases/latest) and download the appropriate 64-bit binary from the directory corresponding to your operating system, or download from the [`release/` folder](https://github.com/Python3-8/sqframe/tree/master/release). Move this binary to some directory in your `$PATH` environment variable, and you're all set.

# Usage
Yesterday (November 8th, 2023) I used SqFrame to edit images for my Instagram, for the first time ever: [A horrible game](https://www.instagram.com/p/CzZDWNTS_qW/?img_index=1)
In order to use SqFrame, first install it, and then run the following command:
```sh
$ sqframe -h
A tool to create a square frame with a blurred background for any image, to match the aspect ratio 1:1

Usage: sqframe [OPTIONS]

Options:
  -i, --input-path <INPUT_PATH>    Input file path, defaults to clipboard
  -o, --output-path <OUTPUT_PATH>  Output file path, defaults to clipboard
  -h, --help                       Print help
  -V, --version                    Print version

```
Everything you need to know is displayed here.
```sh
$ sqframe # reads from the clipboard and saves the edited version to the clipboard
$ sqframe -i /path/to/input-image.png # reads from /path/to/input-image.png and saves the edited version to the clipboard
$ sqframe -o /path/to/output-image.png # reads from the clipboard and saves the edited version to /path/to/output-image.png
$ sqframe -i /path/to/input-image.png -o /path/to/output-image.png # reads from /path/to/input-image.png and saves the edited version to /path/to/output-image.png
```
If `-o /path/to/output-image.png` is specified and there is already a file at `/path/to/output-image.png`, SqFrame backs up this file in a temporary directory to prevent any loss of data. This is done even after the user permits the program to replace the original file.