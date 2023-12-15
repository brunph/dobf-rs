# dobf
**dobf** is a binary patching tool used to primarily deal with obfuscation.\
Current features include:
- Nop instruction optimizing (90 90 becomes 66 90 and so forth)
- Fast pattern matching
- Easily configurable with [`toml`](https://toml.io/en/) configs 

# Building
```
git clone https://github.com/brunph/dobf-rs.git
cargo build --release
```

# Usage
**dobf** works with [`toml`](https://toml.io/en/) to specify the types of patches to be done. Below is an example of such a patch.
```toml
name = "example"

[obfret]
# asm:
# lea     rsp, [rsp+0x8]
# jmp     qword [rsp-0x8]
# ->
# ret
pattern = "48 8D 64 24 08 FF 64 24 F8"
patch = "C3 90 90 90 90 90 90 90 90" # adding these nops are optional, but a good way to get rid of the remaining junk after the patch
order = 0 # when multiple transforms are used, one can specify the order to ensure they are applied correctly
```
Then simply run your compiled version of **dobf**
```
dobf.exe -i test.asm -c example.toml
```
### Before
![](/assets/1.png)
### After
![](/assets/2.png)

# Todo
- Add serialization output (ability to write all patches to a file, so they then can be read into any program of choice)
- Nested matching to improve correctness
- Improve pattern matching by using [`regex`](https://github.com/rust-lang/regex)
