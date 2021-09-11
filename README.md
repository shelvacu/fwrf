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

| m | o | m |
| - | - | - |
| o | r | * |
| - | - | - |
| . | . | . |

Where `.`s are empty spaces, and `*` is the next empty space.

Access the hashmap keys `or` for the row prefix and `m` for the column prefix. Take the intersection of the two sets to get the set of next possible characters.

```
rowSet := rowMap["or"] #=> {'a', 'd', 'e'}
colSet := rowMap["m"] #=> {'b', 'c', 'd', 'e', 'f', ...}
charSet := intersection(rowSet, colSet) #=> {'d', 'e'}
```

Iterate over each possible next character, creating a new matrix with one more character filled in and calling the function with it.

Note that the intersection of the two sets may be empty. That's just a "dead end", where no recursion happens (iterate over the empty set) and it just returns.

### Actual Algorithm Details

In the interest of