#' @include extendr-wrappers.R error-wrapper.R
NULL

#' @export Transaction
NULL

#' @export Origin
NULL

#' @export
`$.Transaction` <- make_dispatcher(Transaction)

#' @export
`[[.Transaction` <- `$.Transaction`

#' @export
`$.Origin` <- make_dispatcher(Origin)

#' @export
`[[.Origin` <- `$.Origin`

install_checked_static(Origin, "new")

#' @export
`==.Origin` <- function(e1, e2) {
  e1$equal(e2)
}

#' @export
`!=.Origin` <- function(e1, e2) {
  e1$not_equal(e2)
}

#' @export
`<.Origin` <- function(e1, e2) {
  e1$less_than(e2)
}

#' @export
`<=.Origin` <- function(e1, e2) {
  e1$less_than_equal(e2)
}

#' @export
`>.Origin` <- function(e1, e2) {
  e1$greater_than(e2)
}

#' @export
`>=.Origin` <- function(e1, e2) {
  e1$greater_than_equal(e2)
}
