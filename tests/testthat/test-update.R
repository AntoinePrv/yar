# TODO work on not using empty update here

test_that("Update print outputs Update(...)", {
  update <- Update$new()
  expect_invisible(print(update))
})

test_that("Update can be formatted", {
  update <- Update$new()
  fmt <- format(update)
  expect_type(fmt, "character")
  expect_length(fmt, 1)
})

test_that("Update$new creates an empty Update", {
  update <- Update$new()
  expect_true(inherits(update, "Update"))
  expect_true(update$is_empty())
})

for (version in c("v1", "v2")) {
  local(
    {
      test_that(paste("Update encode/decode roundtrip", version), {
        update <- Update$new()
        encoded <- update[[paste0("encode_", version)]]()
        expect_true(is.raw(encoded))
        decoded <- Update[[paste0("decode_", version)]](encoded)
        expect_true(decoded$is_empty())
      })
    },
    list(version = version)
  )
}

for (version in c("v1", "v2")) {
  local(
    {
      test_that(paste("Update decode", version, "errors on invalid data"), {
        expect_error(
          Update[[paste0("decode_", version)]](as.raw(c(0xff)))
        )
      })
    },
    list(version = version)
  )
}
