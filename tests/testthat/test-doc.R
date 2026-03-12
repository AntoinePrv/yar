test_that("Doc can be created", {
  doc <- Doc$new()
  expect_true(inherits(doc, "Doc"))
})

test_that("Doc has a positive client_id", {
  doc <- Doc$new()
  expect_true(doc$client_id() > 0)
})

test_that("two Docs have different client_ids", {
  expect_false(Doc$new()$client_id() == Doc$new()$client_id())
})

test_that("Doc has a non-empty guid", {
  doc <- Doc$new()
  expect_true(nchar(doc$guid()) > 0)
})

test_that("two Docs have different guids", {
  expect_false(Doc$new()$guid() == Doc$new()$guid())
})

test_that("Transaction$new returns a Transaction", {
  doc <- Doc$new()
  trans <- Transaction$new(doc)
  expect_true(inherits(trans, "Transaction"))
})

test_that("Text insert and retrieve get_string", {
  doc <- Doc$new()
  text <- doc$get_or_insert_text("article")

  trans <- Transaction$new(doc, mutable = TRUE)
  text$insert(trans, 0L, "hello")
  text$insert(trans, 5L, " world")
  trans$commit()

  expect_equal(text$get_string(trans), "hello world")
  trans$drop()
})

test_that("Multiple readonly transaction does not deadlock", {
  doc <- Doc$new()
  text <- doc$get_or_insert_text("article")

  trans1 <- Transaction$new(doc)
  trans2 <- Transaction$new(doc)
  trans1$drop()
  trans2$drop()
})

test_that("Errors when using Transaction after drop", {
  doc <- Doc$new()
  text <- doc$get_or_insert_text("article")
  trans <- Transaction$new(doc, mutable = TRUE)
  trans$drop()

  expect_s3_class(trans$commit(), "extendr_error")
  expect_s3_class(text$get_string(trans), "extendr_error")
})
