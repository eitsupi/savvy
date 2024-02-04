

<!-- README.md is generated from README.qmd. Please edit that file -->

# Savvy - A simple R extension interface using Rust

<!-- badges: start -->

[![](https://img.shields.io/github/actions/workflow/status/yutannihilation/savvy/R-CMD-check.yaml?style=flat-square&logo=github)](https://github.com/yutannihilation/savvy/actions/workflows/R-CMD-check.yaml)
[![](https://img.shields.io/crates/v/savvy.svg?style=flat-square&logo=rust)](https://crates.io/crates/savvy)
[![](https://img.shields.io/docsrs/savvy.svg?style=flat-square&logo=docsdotrs)](https://docs.rs/savvy/latest/)

<!-- badges: end -->

**savvy** is a simple R extension interface using Rust, like the
[extendr](https://extendr.github.io/) framework. The name “savvy” comes
from the Japanese word “錆” (pronounced as `sàbí`), which means “Rust”.

With savvy, you can automatically generate R functions from Rust code.
This is an example of what savvy-powered function would look like.

**Rust**:

``` rust
use savvy::savvy;

/// Convert to Upper-case
/// 
/// @param x A character vector.
/// @export
#[savvy]
fn to_upper(x: StringSexp) -> savvy::Result<savvy::Sexp> {
    // Use `Owned{type}Sexp` to allocate an R vector for output.
    let mut out = OwnedStringSexp::new(x.len())?;

    for (i, e) in x.iter().enumerate() {
        // To Rust, missing value is an ordinary value. In `&str`'s case, it's just "NA".
        // You have to use `.is_na()` method to distinguish the missing value.
        if e.is_na() {
            // Values need to be set by `set_elt()` one by one.
            out.set_elt(i, <&str>::na())?;
            continue;
        }

        let e_upper = e.to_uppercase();
        out.set_elt(i, e_upper.as_str())?;
    }

    out.into()
}
```

**R**:

``` r
to_upper(c("a", "b", "c"))
#> [1] "A" "B" "C"
```

## User guide

<https://yutannihilation.github.io/savvy/guide/>

## Examples

A toy example R package can be found in [`R-package/`
directory](https://github.com/yutannihilation/savvy/tree/master/R-package).

<details>
<summary>
extendr?
</summary>

## What the hell is this?? Why do you need another framework when there’s extendr?

[extendr](https://extendr.github.io/) is great and ready to use, but
it’s not perfect in some points (e.g., [error
handling](https://github.com/extendr/extendr/issues/278)) and it’s a
kind of stuck; extendr is too feature-rich and complex that no one can
introduce a big breaking change easily. So, I needed to create a new,
simple framework to experiment with. The main goal of savvy is to
provide a simpler option other than extendr, not to be a complete
alternative to extendr.

### Pros and cons compared to extendr

Pros:

- You can use `Result` for error handling instead of `panic!`
- You can compile your package for webR (I hope extendr gets webR-ready
  soon)

Cos:

- savvy prefers explicitness over ergonomics
- savvy provides limited amount of APIs and might not fit for complex
  usages

</details>
