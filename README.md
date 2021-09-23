# FWRF

It stands for [Fresco](https://en.wikipedia.org/wiki/Fresco) Worker's Raging Frosty, or **Fast Word Rectangle Finder** in polite company.

### Build and Run

If you haven't already, install rust nightly with [rustup](https://rustup.rs/).

To allow for more compiler optimizations, **each size of square is a different binary and requires a different command**. The quick version is to just run `./make-bins.sh` to get a bunch of binaries in the `bin` directory. This script builds sizes from 2x2 to 15x15.

Then, if you're searching for a rectangle of a specific size, the binary name is `bin/fwrf-`WIDTH`x`HEIGHT, where WIDTH is always the larger dimension. run `bin/fwrf-5x5 --help` for options, and run with a wordlist to start processing.

If you're searching for *all* sizes of word square, `./run-bins.sh` is a handy script to run all the binaries for each size from 2x2 to 15x15. It passes all options to each `fwrf` binary.

### Manual build/Features

The default features are designed to make development and testing easier. To build your own binary, you need the following features:

    * `width-X`, where X is a number between 2 and 15, such as `width-5`
    * `height-X`, where X is a number between 2 and 15, such as `height-6`
    * Exactly one of `charset-english-extended` or `charset-english-small`. "Small" includes letters a-z, a few symbols, and 'é'; "Extended" includes letters a-z, numerals 0-9, a few letters with diacritics, and more symbols
    * `square` **if and only if** width and height are the same. This is needed due to some limitations in rust's const generics.

Additionally, there is one optional feature:

    * `unchecked`, which enables a lot of unsafe code but should allow for more compiler optimizations. If the program runs with no panics while this feature is off, then it should run without any UB when this feature is on.
    * `do-debug`, which enables some (very noisy) output only intended for debugging purposes.

So, to build a binary using the small english character set to find 5x8 word rectangles with unsafe code enabled, run:

    cargo +nightly build --release --no-default-features --features width-8,height-5,charset-english-small,unchecked

-----

### Word Squares

A word square is a grid (matrix) of letters that make words along both the column and the rows.

```
H E A R T
E M B E R
A B U S E
R E S I N
T R E N D
```

Of course, the words need not be the same in both directions, such as:

```
P O M A D E
A R I S E N
R A N K L E
I N D I U M
A G E N D A
H E D G E S
```

[Pomade](https://en.wikipedia.org/wiki/Pomade) is a type of hair gel, to "rankle" is to "cause annoyance or resentment that persists", and [Indium](https://en.wikipedia.org/wiki/Indium) is a chemical element.

[The wikipedia article on word squares](https://en.wikipedia.org/wiki/Word_square) calls these "word squares" and "double word squares" respectively.

Those are bad names. There is nothing double about it. They are symmetric and asymmetric word squares.

### Word Rectangles

If the words in the rows and columns do not need to be the same, then the height and width of the matrix don't need to match either:

```
A B S O R B E D
P R O P E R L Y
R E N E G A D E
E N G R A V E R
S T E A L E R S
```

I call this a "word rectangle". Just as the square shape is a special case of the rectangle, the word square is a special case of the word rectangle.

### Theoretical Algorithm Overview

First, we take in a wordlist and build a mapping of word prefixes to the set of characters that could come next. (This could be a `HashMap<String,HashSet<char>>` or a [Trie](https://en.wikipedia.org/wiki/Trie) or something else.)

For every word of the correct length (the width of the square): iterate over each index `i`:

* Create a prefix string of all characters before `i` (possibly empty)
* Access the value in the hashmap with the prefix string as a key. Create it as an empty set if it doesn't already exist
* Add the character at `i` to the set retrieved from the hashmap

If the width of the desired rectangle is not the same as the height, repeat the process for words matching the height of the square. I'll call these the "RowMap" and "ColMap" respectively. If the desired rectangle is a square, they are the same map.

To do the actual solving, we have a recursive function which takes in a partially completed, potential word matrix and iterates through the possible characters in one cell, and then recurses.

For example, given a partially completed matrix like this:

| M | O | M |
|:-:|:-:|:-:|
| O | R | * |
| . | . | . |
| . | . | . |

Where `.`s are empty spaces, and `*` is the next empty space.

Access the hashmap keys `or` for the row prefix and `m` for the column prefix. Take the intersection of the two sets to get the set of next possible characters.

```
rowSet := rowMap["or"] #=> {'a', 'd', 'e'}
colSet := colMap["m"] #=> {'b', 'c', 'd', 'e', 'f', ...}
charSet := intersection(rowSet, colSet) #=> {'d', 'e'}
```

Iterate over each possible next character, creating a new matrix with one more character filled in and calling the function with it.

Note that the intersection of the two sets may be empty. That's just a "dead end", where no recursion happens (iterate over the empty set) and it just returns.

### Actual Implementation Details

In the interest of SPEED, the actual code works a bit differently. The `CharSet` type is a HashSet in spirit, but a `u32`-backed bitmap in practice (thus set intersection becomes a bitwise and). This means that there is a limit of 32 possible values. These are represented as 0..32 in a `u8`, in a wrapper called `EncodedChar`. There's also a special null value, 255. An `Option<_>` is not used because:

1. I want the value to fit in one byte.
2. I want to be able to set the Nth bit in a CharSet(u32) without an extra addition step, which means 0 must be used for one of the meaningful values, and so I cannot use a `NonZeroU8`.

Thus you will see `... == NULL_CHAR` rather than the more rusty `.is_none()`.

Liberal use is made of static-size arrays and ranged ints to make safe unchecked indexes into them.

For the prefix map, static arrays of the word length are used with the remaining elements filled in with `NULL_CHAR`, so `prefix_map.get(&[EncodedChar('a'), NULL_CHAR, NULL_CHAR])` gets the `CharSet` of next possible characters for all words that start with 'a' in a 3-letter-word prefix map.

The `compute` function is "flattened out", so no recursion happens and it's just a simple loop. You can think of the working matrix and associated list of CharSets as the stack, and `at_idx` as the stack pointer.

### `--must-include` Implementation Details

Part of the goal of adding `--must-include` was to make the search much, much faster by not bothering to search matrixes that couldn't possibly contain the `must-include` words.

First, we build a list of "templates", which is every way the entire set of `must-include` words can be arranged onto the matrix. Take for example SEWER and WET, which can be arranged three ways on a 5x3:

```
1.
SEWER
**E**
**T**

2.
*W***
SEWER
*T***

3.
***W*
SEWER
***T*
```

Then, for each template a different prefix map is created, although the name "prefix map" doesn't make as much sense anymore. This is because the key can include letters *after* the prefix. Thus, `some_prefix_map[NULL_CHAR, NULL_CHAR, EncodedChar('t')]` returns a CharSet of all letters that could 'fill in' the first NULL_CHAR, which is the set of all first characters of {3-letter words that end with 't'}.

Words are added to the prefix map only if they "fit" in one of the rows or columns of the template matrix.

Thus, prefix maps can be significantly smaller, which is particularly helpful if it allows it to fit in L2 cache rather than L3 cache, for example.

Prefix maps can also be slightly larger; This is necessary to keep the same speed and allow the hot path to use nearly the same algorithm.

When calling `compute`, each template/prefix map is passed in turn. `compute` keeps a copy of the matrix originally passed in, and "skips over" any values that aren't `NULL_CHAR` in the original matrix.

## Previous Iterations

Previous (uglier, hackier) versions of this project:

* [rust-word-squares](https://github.com/shelvacu/rust-word-squares), written in Rust, much less use of well-needed encapsulation. Possibly broken. Can find word rectangles, despite the name.
* [fast-word-squares](https://github.com/shelvacu/fast-word-squares), written in [Crystal](https://crystal-lang.org/) before Crystal 1.0, so likely uses broken/deprecated features (including turning off the GC during the hot code, which is a big no-no but garnered a 3% speedup at the time). The first version I made, supports a client/server architecture but does not support multithreading. Can only find word squares.