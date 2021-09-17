# FWRF

It stands for [Fresco](https://en.wikipedia.org/wiki/Fresco) Worker's Raging Frosty, or **Fast Word Rectangle Finder** in polite company.

## Info

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

|:M:|:O:|:M:|
|:O:|:R:|:*:|
|:.:|:.:|:.:|
|:.:|:.:|:.:|

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

## Previous Iterations

Previous (uglier, hackier) versions of this project:

* [rust-word-squares](https://github.com/shelvacu/rust-word-squares), written in Rust, much less use of well-needed encapsulation. Possibly broken. Can find word rectangles, despite the name.
* [fast-word-squares](https://github.com/shelvacu/fast-word-squares), written in [Crystal](https://crystal-lang.org/) before Crystal 1.0, so likely uses broken/deprecated features (including turning off the GC during the hot code, which is a big no-no but garnered a 3% speedup at the time). The first version I made, supports a client/server architecture but does not support multithreading. Can only find word squares.