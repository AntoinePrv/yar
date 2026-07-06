#' @include extendr-wrappers.R
NULL

# Rust-side convention: any `#[extendr]` method named `PRINT_METHOD` returns the
# object's textual representation (akin to Python's `__repr__`). Such a method is
# registered automatically as the S3 `print` method for its class in `.onLoad()`,
# so Rust owns the formatting and no per-class R code is required.
PRINT_METHOD <- "__yr_print__"

# Shared S3 `format`/`print` methods: format returns the Rust-owned string,
# print emits it. `[[` binds `self` before calling the `PRINT_METHOD` binding.
format_yr <- function(x, ...) {
  x[[PRINT_METHOD]]()
}

print_yr <- function(x, ...) {
  cat(format(x, ...), "\n", sep = "")
  invisible(x)
}

# Register `print`/`format` for every extendr class exposing a `PRINT_METHOD`
# binding, so a `__yr_print__` method on the Rust side is all that's needed.
.onLoad <- function(libname, pkgname) {
  ns <- asNamespace(pkgname)
  for (name in ls(ns, all.names = TRUE)) {
    obj <- get0(name, envir = ns, inherits = FALSE)
    if (
      is.environment(obj) &&
        exists(PRINT_METHOD, envir = obj, inherits = FALSE)
    ) {
      registerS3method("print", name, print_yr, envir = ns)
      registerS3method("format", name, format_yr, envir = ns)
    }
  }
}
