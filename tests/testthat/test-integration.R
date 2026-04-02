# This is the quick start example from yrs, https://docs.rs/yrs/latest/yrs/
for (version in c("v1", "v2")) {
  local(
    {
      test_that(paste("Synchronize two docs", version), {
        doc <- yar::Doc$new()
        text <- doc$get_or_insert_text("article")

        doc$with_transaction(
          function(trans) {
            text$insert(trans, 0L, "hello")
            text$insert(trans, 5L, " world")
            trans$commit()

            expect_equal(text$get_string(trans), "hello world")
          },
          mutable = TRUE
        )

        # Synchronize state with remote replica
        remote_doc <- yar::Doc$new()
        remote_text <- remote_doc$get_or_insert_text("article")

        remote_sv_raw <- remote_doc$with_transaction(function(remote_trans) {
          remote_trans$state_vector()[[paste0("encode_", version)]]()
        })

        # Get update with contents not observed by remote_doc
        update <- doc$with_transaction(function(local_trans) {
          remote_sv <- yar::StateVector[[paste0("decode_", version)]](
            remote_sv_raw
          )
          local_trans[[paste0("encode_diff_", version)]](remote_sv)
        })

        # Apply update on remote doc
        remote_doc$with_transaction(
          function(remote_trans) {
            remote_trans[[paste0("apply_update_", version)]](update)
            remote_trans$commit()

            expect_equal(remote_text$get_string(remote_trans), "hello world")
          },
          mutable = TRUE
        )
      })
    },
    list(version = version)
  )
}

####################
# Observer pattern #
####################

test_that("Observers fire only on the directly modified type with correct events", {
  doc <- Doc$new()
  root <- doc$get_or_insert_map("root")

  # Build nested structure:
  #   root (Map) -> "items" (Array) -> [0] text_a (Text)
  #                 "label" (Text)  — sibling text under the same map
  items <- NULL
  text_a <- NULL
  text_b <- NULL
  doc$with_transaction(
    function(trans) {
      items <<- root$insert_array(trans, "items")
      text_a <<- items$insert_text(trans, 0L)
      text_b <<- root$insert_text(trans, "label")
    },
    mutable = TRUE
  )

  map_event <- NULL
  array_event <- NULL
  text_a_event <- NULL
  text_b_event <- NULL

  root$observe(
    function(trans, event) {
      map_event <<- list(keys = event$keys(trans))
    },
    key = 1L
  )
  items$observe(
    function(trans, event) {
      array_event <<- list(delta = event$delta(trans))
    },
    key = 2L
  )
  text_a$observe(
    function(trans, event) {
      text_a_event <<- list(delta = event$delta(trans))
    },
    key = 3L
  )
  text_b$observe(
    function(trans, event) {
      text_b_event <<- list(delta = event$delta(trans))
    },
    key = 4L
  )

  # Modify text_a — only its observer fires
  doc$with_transaction(
    function(trans) text_a$push(trans, "hello"),
    mutable = TRUE
  )
  expect_null(map_event)
  expect_null(array_event)
  expect_equal(
    text_a_event$delta,
    list(list(inserted = "hello", attributes = NULL))
  )
  expect_null(text_b_event)

  # Modify sibling text_b — only its observer fires
  text_a_event <- NULL
  doc$with_transaction(
    function(trans) text_b$push(trans, "world"),
    mutable = TRUE
  )
  expect_null(map_event)
  expect_null(array_event)
  expect_null(text_a_event)
  expect_equal(
    text_b_event$delta,
    list(list(inserted = "world", attributes = NULL))
  )

  # Modify array — only array observer fires
  text_a_event <- NULL
  text_b_event <- NULL
  doc$with_transaction(
    function(trans) items$insert_any(trans, 1L, 42L),
    mutable = TRUE
  )
  expect_null(map_event)
  expect_equal(
    array_event$delta,
    list(list(retain = 1L), list(added = list(42L)))
  )
  expect_null(text_a_event)
  expect_null(text_b_event)

  # Modify map — only map observer fires
  array_event <- NULL
  doc$with_transaction(
    function(trans) root$insert_any(trans, "new_key", "value"),
    mutable = TRUE
  )
  expect_equal(map_event$keys, list(new_key = list(inserted = "value")))
  expect_null(array_event)
  expect_null(text_a_event)
  expect_null(text_b_event)
})

test_that("Unobserving one nested type does not affect sibling observers", {
  doc <- Doc$new()
  root <- doc$get_or_insert_map("root")

  text <- NULL
  arr <- NULL
  doc$with_transaction(
    function(trans) {
      text <<- root$insert_text(trans, "t")
      arr <<- root$insert_array(trans, "a")
    },
    mutable = TRUE
  )

  text_deltas <- list()
  arr_deltas <- list()
  text$observe(
    function(trans, event) {
      text_deltas[[length(text_deltas) + 1L]] <<- event$delta(trans)
    },
    key = 1L
  )
  arr$observe(
    function(trans, event) {
      arr_deltas[[length(arr_deltas) + 1L]] <<- event$delta(trans)
    },
    key = 2L
  )

  # Both fire
  doc$with_transaction(
    function(trans) {
      text$push(trans, "x")
      arr$insert_any(trans, 0L, 1L)
    },
    mutable = TRUE
  )
  expect_length(text_deltas, 1L)
  expect_equal(
    text_deltas[[1L]],
    list(list(inserted = "x", attributes = NULL))
  )
  expect_length(arr_deltas, 1L)
  expect_equal(
    arr_deltas[[1L]],
    list(list(added = list(1L)))
  )

  # Unobserve text only
  text$unobserve(key = 1L)

  doc$with_transaction(
    function(trans) {
      text$push(trans, "y")
      arr$insert_any(trans, 1L, 2L)
    },
    mutable = TRUE
  )
  expect_length(text_deltas, 1L) # unchanged — still just the first
  expect_length(arr_deltas, 2L) # still fires
  expect_equal(
    arr_deltas[[2L]],
    list(list(retain = 1L), list(added = list(2L)))
  )
})
