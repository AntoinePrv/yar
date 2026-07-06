test_that("Prelim$detect identifies type from R object", {
  expect_true(Prelim$detect("hello")$is_text())
  expect_true(Prelim$detect(list(a = 1L, b = 2L))$is_map())
  expect_true(Prelim$detect(list(1L, 2L, 3L))$is_array())
  expect_true(Prelim$detect(42L)$is_any())

  # Detect on an existing Prelim preserves type
  expect_true(Prelim$detect(Prelim$detect("hi"))$is_text())
})

test_that("Prelim constructors create correct types", {
  expect_true(Prelim$text("hi")$is_text())
  expect_true(Prelim$text()$is_text())
  expect_true(Prelim$array(list(1L, 2L))$is_array())
  expect_true(Prelim$array(list())$is_array())
  expect_true(Prelim$array()$is_array())
  expect_true(Prelim$map(list(a = 1L))$is_map())
  expect_true(Prelim$map(list())$is_map())
  expect_true(Prelim$map()$is_map())
  expect_true(Prelim$any(42L)$is_any())
})

test_that("Prelim recursive flag applies only to array/map", {
  expect_false(Prelim$detect(list(1L, 2L))$is_recursive())
  expect_true(Prelim$detect(list(1L, 2L), recursive = TRUE)$is_recursive())
  expect_true(Prelim$detect(list(a = 1L), recursive = TRUE)$is_recursive())
  expect_false(Prelim$text("hi")$is_recursive())
  expect_false(Prelim$any(1L)$is_recursive())
})

test_that("Prelim can be inserted into a Map via Doc transaction", {
  doc <- Doc$new()
  map <- doc$get_or_insert_map("data")

  doc$with_transaction(
    function(trans) {
      map$insert(trans, "greeting", Prelim$detect("hello"))
      map$insert(trans, "nums", Prelim$detect(list(1L, 2L, 3L)))
      map$insert(trans, "nested_map", Prelim$map(list(a = 1L, b = 2L)))
      map$insert(trans, "nested_arr", Prelim$array(list(1L, 2L)))

      expect_equal(map$len(trans), 4L)
      expect_setequal(
        map$keys(trans),
        c("greeting", "nums", "nested_map", "nested_arr")
      )
    },
    mutable = TRUE
  )
})

test_that("Prelim prints its contents", {
  expect_match(
    format(Prelim$text("hello")),
    '^Text\\(.*Inserted\\(Any\\(String\\("hello"\\)\\)'
  )
  expect_match(
    format(Prelim$array(list(1L, 2L))),
    "^Array\\(ArrayPrelim\\(\\[.*Number\\(1"
  )
  expect_match(
    format(Prelim$map(list(a = 1L, b = 2L))),
    '^Map\\(MapPrelim\\(\\{.*"a"'
  )
  expect_match(format(Prelim$any(42L)), "^Any\\(Number\\(42")
})
