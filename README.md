# globgroups
Expands glob groups like `foo{bar,baz}` -> `["foobar", "foobaz"]`

Does not (currently) support wildcards like `*.txt`,
because those are context-sensitive.

## Examples
```rust
use globgroups::GlobExpr;

fn simple() {
    let glob: GlobExpr = "foo-{bar,baz}-suffix".parse().unwrap();
    assert_eq!(
        glob.expand().collect::<Vec<String>>(),
        vec![
            "foo-bar-suffix",
            "foo-baz-suffix"
        ]
    )
}
```

## Notes
There is a pure-python version of this library [globgroups.py](https://github.com/Techcable/globgroups.py).

There is an older (and much buggier) single-file python version in `misc/globgroups.py`
