# Rust Performance Guidelines

## Intro

This is a quick guideline to avoid the most common performance pitfalls in rust.
Although performance is important, it's not the only thing to consider when writing code. This guideline should be closely applied only in hot-paths, where performance is crucial, and more loosely applied in cold-paths, where readability and maintainability are more important.

## Memory Allocations
Frequent memory allocations can degrade performance either for the sheer number of allocations or for the consequent memory fragmentation. Most of the guidelines here are to avoid unnecessary allocations and copies. For Ockam Command, the typical memory fragmentation symptoms are unstable or degraded performance.

## Tools
There are some tools that can help you to profile Ockam Command portals. See [here](tools/profile/README.md) for more information.

## Guidelines

### Owned Parameters
If the function needs an owned copy of the element, ask an owned instance directly in the parameter. This will allow the caller to decide if it wants to clone the element or give its own.

```rust
// Bad
fn example(item: &Type) {
  self.item = item.clone();
}

// Good
fn example(item: Type) {
  self.item = item;
}
```

### Returning a reference
Avoid returning a cloned value when a reference is enough. This will avoid unnecessary allocations. It'll be the caller's responsibility to clone the value when needed.

```rust
// Bad
fn get_value(&self) -> Type {
  self.value.clone()
}

// Good
fn get_value(&self) -> &Type {
  &self.value
}
```

### Borrowed Parameters
If the function doesn't need to own the instance, ask for a borrowed instance in the parameter.

```rust
// Bad
fn example(item: Type) {
  self.map.get(&item);
}

// Good
fn example(item: &Type) {
  self.map.get(item);
}
```

### Allow direct field access
By allowing direct access to a structure fields, it can be "deconstructed" and used directly, avoiding unnecessary allocations.

```rust
// Bad
struct MyStruct {
    field: Type,
}

impl MyStruct {
    fn get_field(&self) -> &Type {
        &self.field
    }
}

fn example(my_struct: MyStruct) {
    let field = my_struct.get_field().clone();
    do_something(field);
}

// Good
struct MyStruct {
    pub field: Type,
}

fn example(my_struct: MyStruct) {
    do_something(my_struct.field);
}
```

### `ok_or` Errors allocation
Memory allocations for errors should only happen when an error is actually returned. To avoid unnecessary allocations, use `ok_or_else` instead of `ok_or`.

```rust
// Bad
let value = some_function().ok_or(Error::new("Error"));

// Good
let value = some_function().ok_or_else(|| Error::new("Error"));
```

### Reuse in loops
When possible, reuse instances in loops to avoid unnecessary allocations.

```rust
// Bad
loop {
  let value = Vec::new();
  // do something with value
}

// Good
let mut value = Vec::new();
loop {
  value.clear();
  // do something with value
}
```

### Pre-compute the size
Not providing the size to a vector will cause re-allocations. A re-allocation is a request to enlarge the existing allocation, when it's not possible it'll cause a copy of the existing data to a new allocation. This is exponentially expensive as the size grows. To avoid this, pre-compute the size of the vector when possible, Cbor structures have the possibility to pre-compute the size before serialization.

```rust
// Bad
let mut vec = Vec::new();
for i in 0..100 {
  vec.push(i);
}

// Good
let mut vec = Vec::with_capacity(100);
for i in 0..100 {
  vec.push(i);
}
```

However, an empty `Vec` has zero capacity and doesn't trigger an allocation until the first element is pushed. If you first need to create a `Vec` and then populate it, use `reserve` to pre-allocate the necessary space.

```rust
struct MyStruct {
    vec: Vec<i32>,
}

// Bad
impl MyStruct {
    fn new() -> Self {
        MyStruct {
            // no need to pre-allocate here
            vec: Vec::with_capacity(100),
        }
    }

    fn populate(&mut self) {
        for i in 0..100 {
            self.vec.push(i);
        }
    }
}

// Good
impl MyStruct {
    fn new() -> Self {
        MyStruct {
            // no extra allocation here
            vec: Vec::new(),
        }
    }

    fn populate(&mut self) {
        self.vec.reserve(100);
        for i in 0..100 {
            self.vec.push(i);
        }
    }
}
```

### Avoid `async` Traits
The `async_trait` implementation allocates a future every time a method is called.
When an `async_trait` is being called very frequently (100+/s), consider the usage of synchronous traits, or a template instead.

### Cow
Sometimes when a function needs to return a value that can be either owned or borrowed, `Cow` can be used to avoid cloning the borrowed value. This also requires the usage of a lifetime and may increase the complexity of the code, use it only when necessary.

```rust
// Bad
struct MyStruct {
    value: Type,
}

fn create_struct_owned(value: Type) -> MyStruct {
    MyStruct {
        value,
    }
}
fn create_struct_borrowed(value: &Type) -> MyStruct {
    MyStruct {
        value: value.clone(),
    }
}

// Good
struct MyStruct<'a> {
    value: Cow<'a, Type>,
}

fn create_struct_owned(value: Type) -> MyStruct {
    MyStruct {
        value: Cow::Owned(value),
    }
}

fn create_struct_borrowed(value: &Type) -> MyStruct {
    MyStruct {
        value: Cow::Borrowed(value),
    }
}
```

### Avoid overly complex data structures
`HashMap`, `HashSet`, and other complex data structures can be expensive in terms of memory and relative slow. When the number of elements is small (10s), consider using a simple vector instead.

```rust
// Bad
struct MyStruct {
    plugins: HashMap<String, Type>,
}

impl MyStruct {
    fn new() -> Self {
        let mut plugins = HashMap::new();
        plugins.insert("plugin1".to_string(), Type::new());
        plugins.insert("plugin2".to_string(), Type::new());
        plugins.insert("plugin3".to_string(), Type::new());

        MyStruct {
            plugins
        }
    }
}

// Good
struct MyStruct {
    plugins: Vec<(String, Type)>,
}

impl MyStruct {
    fn new() -> Self {
        let plugins = vec![
            ("plugin1".to_string(), Type::new()),
            ("plugin2".to_string(), Type::new()),
            ("plugin3".to_string(), Type::new()),
        ];

        MyStruct {
            plugins
        }
    }
}
```
