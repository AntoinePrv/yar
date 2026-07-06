#' @include extendr-wrappers.R
NULL

# Rust-side convention: any `#[extendr]` method returning `Result<_, Error>`
# is suffixed with `EC_SUFFIX`. extendr returns the Err as an R condition
# object, which is silently discarded on void-returning calls. The helpers
# below rewrite those methods to `stop()` on error, so failures surface as
# native R errors.
EC_SUFFIX <- "_ec"

#' Raise condition objects as R errors
#'
#' Inspects a value returned by a fallible extendr binding. If it is a
#' condition (extendr's representation of `Err`), it is raised with
#' [stop()]; otherwise the value is returned unchanged.
#'
#' @param res Return value of an `_ec`-suffixed extendr call.
#' @return `res`, invisibly, when it is not a condition.
#' @keywords internal
#' @noRd
throw_if_condition <- function(res) {
  if (inherits(res, "condition")) {
    stop(res)
  }
  res
}

#' Build a checked `$` dispatcher for an extendr class environment
#'
#' Returns a replacement for the generated `$.<Class>` S3 method. The
#' dispatcher looks up `name` in `env`; if it is missing but `<name>_ec`
#' exists, the fallible binding is invoked and its result passed through
#' `throw_if_condition()`. The wrapper is rebuilt on every call because
#' the generated dispatcher injects `self` by rewriting the function's
#' closure environment, which would clobber any captured state set up
#' once at install time.
#'
#' @param env Environment holding the class's generated bindings (e.g.
#'   `Transaction`, `Origin`).
#' @return A function with signature `function(self, name)` suitable for
#'   assignment to `` `$.<Class>` ``.
#' @keywords internal
#' @noRd
make_dispatcher <- function(env) {
  function(self, name) {
    raw_name <- if (exists(name, envir = env, inherits = FALSE)) {
      name
    } else {
      paste0(name, EC_SUFFIX)
    }
    func <- env[[raw_name]]
    environment(func) <- environment()
    if (!endsWith(raw_name, EC_SUFFIX)) {
      return(func)
    }
    function(...) throw_if_condition(func(...))
  }
}

#' Install a checked alias for a static `_ec` binding
#'
#' Static constructors are called directly on the class environment
#' (e.g. `Origin$new(...)`), bypassing the `$.<Class>` S3 dispatcher,
#' so they need their own bare-name alias. This installs `env[[name]]`
#' as a wrapper that forwards to `env[[paste0(name, "_ec")]]` and runs
#' the result through `throw_if_condition()`.
#'
#' @param env Environment holding the class's generated bindings.
#' @param name Bare (un-suffixed) name to expose. The fallible binding
#'   `paste0(name, "_ec")` must already exist in `env`.
#' @return Invoked for its side effect of mutating `env`.
#' @keywords internal
#' @noRd
install_checked_static <- function(env, name) {
  raw <- env[[paste0(name, EC_SUFFIX)]]
  env[[name]] <- function(...) throw_if_condition(raw(...))
}
