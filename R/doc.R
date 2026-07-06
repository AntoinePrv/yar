#' @include extendr-wrappers.R error-wrapper.R
NULL

#' @export Doc
NULL

#' @export
`$.Doc` <- make_dispatcher(Doc)

#' @export
`[[.Doc` <- `$.Doc`

#' @export
`$.TransactionCleanupEvent` <- make_dispatcher(TransactionCleanupEvent)

#' @export
`[[.TransactionCleanupEvent` <- `$.TransactionCleanupEvent`

Doc$with_transaction <- function(callback, mutable = FALSE, origin = NULL) {
  transaction <- Transaction$lock(self, mutable = mutable, origin = origin)
  on.exit(transaction$unlock())
  callback(transaction)
}
