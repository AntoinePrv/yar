###############
# Transaction #
###############

test_that("Transaction$lock returns a Transaction", {
  doc <- Doc$new()
  trans <- Transaction$lock(doc)
  expect_true(inherits(trans, "Transaction"))
})

test_that("Multiple readonly transaction does not deadlock", {
  doc <- Doc$new()
  text <- doc$get_or_insert_text("article")

  trans1 <- Transaction$lock(doc)
  trans2 <- Transaction$lock(doc)
  expect_true(inherits(trans1, "Transaction"))
  expect_true(inherits(trans2, "Transaction"))
  trans1$unlock()
  trans2$unlock()
})

test_that("Errors when using Transaction after unlock", {
  doc <- Doc$new()
  text <- doc$get_or_insert_text("article")
  trans <- Transaction$lock(doc, mutable = TRUE)
  trans$unlock()

  expect_error(trans$commit())
  expect_error(text$get_string(trans))
})

test_that("Transaction accepts origin", {
  doc <- Doc$new()
  doc$with_transaction(
    function(trans) {
      expect_null(trans$origin())
    },
    mutable = TRUE
  )

  origin <- Origin$new("my-id")
  doc$with_transaction(
    function(trans) {
      o <- trans$origin()
      expect_true(inherits(o, "Origin"))
      expect_true(o == origin)
    },
    mutable = TRUE,
    origin = origin
  )
})

test_that("Transaction state_vector of empty doc is empty", {
  doc <- Doc$new()
  doc$with_transaction(function(trans) {
    sv <- trans$state_vector()
    expect_true(sv$is_empty())
  })
})

for (version in c("v1", "v2")) {
  local(
    {
      test_that(
        paste(
          "Transaction encode_diff and encode_state_as_update",
          version,
          "against current state vector returns raw"
        ),
        {
          doc <- Doc$new()
          text <- doc$get_or_insert_text("article")

          doc$with_transaction(
            function(trans) {
              text$insert(trans, 0L, "hello")
              trans$commit()

              sv <- trans$state_vector()
              diff <- trans[[paste0("encode_diff_", version)]](sv)
              expect_true(is.raw(diff))
              update <- trans[[paste0("encode_state_as_update_", version)]](sv)
              expect_true(is.raw(update))
            },
            mutable = TRUE
          )
        }
      )
    },
    list(version = version)
  )
}

for (version in c("v1", "v2")) {
  local(
    {
      test_that(paste("apply_update", version, "errors on invalid data"), {
        doc <- Doc$new()
        doc$with_transaction(
          function(trans) {
            expect_error(
              trans[[paste0("apply_update_", version)]](as.raw(c(0xff)))
            )
          },
          mutable = TRUE
        )
      })
    },
    list(version = version)
  )
}

##########
# Origin #
##########

test_that("Origin can be created and compared with byte types", {
  o1 <- Origin$new(32)
  expect_true(inherits(o1, "Origin"))
  expect_true(o1 == o1)
  expect_true(o1 <= o1)

  o2 <- Origin$new("my-id")
  expect_true(inherits(o2, "Origin"))
  expect_false(o2 == o1)
  expect_false(o2 < o1)
  expect_false(o1 > o2)

  o3 <- Origin$new(charToRaw("my-id"))
  expect_true(inherits(o3, "Origin"))
  expect_false(o3 == o1)
  expect_true(o3 == o2)

  o4 <- Origin$new(o2)
  expect_true(inherits(o4, "Origin"))
  expect_true(o4 == o2)
})

test_that("Origin cannot be created with invalid types", {
  expect_error(Origin$new(3.14))
  expect_error(Origin$new(TRUE))
  expect_error(Origin$new(NA))
  expect_error(Origin$new(NULL))
})

test_that("Origin can be printed", {
  origin <- Origin$new("my-id")
  expect_output(print(origin), "Origin\\([0-9a-f]+\\)")
})

test_that("Origin can be formatted", {
  origin <- Origin$new("my-id")
  expect_match(format(origin), "Origin\\([0-9a-f]+\\)")
})

test_that("Origin has hex and byte representation", {
  origin <- Origin$new(0x0fa90b)
  # Matches the repr in to_string
  expect_equal(origin$to_hex(), "00000000000fa90b")
  expect_equal(
    origin$to_bytes(),
    as.raw(c(0x00, 0x00, 0x00, 0x00, 0x00, 0x0f, 0xa9, 0x0b))
  )
})
