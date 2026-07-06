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

test_that("Doc print outputs Doc(id: ..., guid: ...)", {
  doc <- Doc$new()
  expect_output(print(doc), "^Doc\\(id: \\d+, guid: .+\\)$")
})

test_that("Doc format returns Doc(id: ..., guid: ...)", {
  doc <- Doc$new()
  expect_match(format(doc), "^Doc\\(id: \\d+, guid: .+\\)$")
})

for (item in list(
  list(method = "get_or_insert_text", class = "TextRef"),
  list(method = "get_or_insert_map", class = "MapRef"),
  list(method = "get_or_insert_array", class = "ArrayRef")
)) {
  local(
    {
      test_that(paste("Doc", item$method, "returns", item$class), {
        doc <- Doc$new()
        obj <- doc[[item$method]]("root")
        expect_true(inherits(obj, item$class))
      })
    },
    list(item = item)
  )
}
