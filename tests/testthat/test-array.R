test_that("Array remove decreases len", {
  doc <- Doc$new()
  arr <- doc$get_or_insert_array("data")

  doc$with_transaction(
    function(trans) {
      arr$insert_any(trans, 0L, "a")
      arr$insert_any(trans, 1L, "b")
      arr$remove(trans, 0L)

      expect_equal(arr$len(trans), 1L)
      expect_equal(arr$get(trans, 0L), "b")
    },
    mutable = TRUE
  )
})

test_that("ArrayRef insert methods return usable nested types", {
  doc <- Doc$new()
  map <- doc$get_or_insert_map("data")

  doc$with_transaction(
    function(trans) {
      arr <- map$insert_array(trans, "root")
      expect_true(inherits(arr, "ArrayRef"))

      arr$insert_any(trans, 0L, "hello")
      arr$insert_any(trans, 1L, 1.5)
      arr$insert_any(trans, 2L, 42L)
      arr$insert_any(trans, 3L, TRUE)
      expect_equal(arr$get(trans, 0L), "hello")
      expect_equal(arr$get(trans, 1L), 1.5)
      expect_equal(arr$get(trans, 2L), 42L)
      expect_equal(arr$get(trans, 3L), TRUE)

      text <- arr$insert_text(trans, 4L)
      expect_true(inherits(text, "TextRef"))
      expect_true(inherits(arr$get(trans, 4L), "TextRef"))
      text$push(trans, "hello")
      text$push(trans, " world")
      expect_equal(text$get_string(trans), "hello world")

      nested_arr <- arr$insert_array(trans, 5L)
      expect_true(inherits(nested_arr, "ArrayRef"))
      expect_true(inherits(arr$get(trans, 5L), "ArrayRef"))
      nested_arr$insert_any(trans, 0L, 42L)
      expect_equal(nested_arr$len(trans), 1L)

      nested_map <- arr$insert_map(trans, 6L)
      expect_true(inherits(nested_map, "MapRef"))
      expect_true(inherits(arr$get(trans, 6L), "MapRef"))
      nested_map$insert_any(trans, "k", TRUE)
      expect_equal(nested_map$len(trans), 1L)

      expect_equal(arr$len(trans), 7L)
      expect_null(arr$get(trans, 99L))
    },
    mutable = TRUE
  )
})

####################
# Observer pattern #
####################

test_that("Array observe callback can read current state via transaction", {
  doc <- Doc$new()
  arr <- doc$get_or_insert_array("data")

  called <- FALSE
  observed_len <- NULL
  observed_target_len <- NULL
  observed_path <- NULL
  observed_delta <- NULL
  arr$observe(
    function(trans, event) {
      called <<- TRUE
      observed_len <<- arr$len(trans)
      observed_target_len <<- event$target()$len(trans)
      observed_path <<- event$path()
      observed_delta <<- event$delta(trans)
    },
    key = 1L
  )

  doc$with_transaction(
    function(trans) arr$insert_any(trans, 0L, 42L),
    mutable = TRUE
  )

  expect_true(called)
  expect_equal(observed_len, 1L)
  expect_equal(observed_target_len, 1L)
  expect_equal(observed_path, list())
  expect_true(is.list(observed_delta))
})

test_that("Array unobserve stops callback from firing", {
  doc <- Doc$new()
  arr <- doc$get_or_insert_array("data")

  count <- 0L
  arr$observe(
    function(trans, event) count <<- count + 1L,
    key = 1L
  )

  doc$with_transaction(
    function(trans) arr$insert_any(trans, 0L, 1L),
    mutable = TRUE
  )
  expect_equal(count, 1L)

  arr$unobserve(key = 1L)

  doc$with_transaction(
    function(trans) arr$insert_any(trans, 0L, 2L),
    mutable = TRUE
  )
  expect_equal(count, 1L)
})

test_that("Array observe callback transaction cannot be used after callback returns", {
  doc <- Doc$new()
  arr <- doc$get_or_insert_array("data")

  captured_trans <- NULL
  captured_event <- NULL
  arr$observe(
    function(trans, event) {
      captured_trans <<- trans
      captured_event <<- event
    },
    key = 1L
  )

  doc$with_transaction(
    function(trans) arr$insert_any(trans, 0L, 1L),
    mutable = TRUE
  )

  # Captured objects are invalidated
  expect_s3_class(
    arr$len(captured_trans),
    "extendr_error"
  )
  expect_s3_class(
    captured_event$path(),
    "extendr_error"
  )
})
