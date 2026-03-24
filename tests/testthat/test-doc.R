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

for (item in list(
  list(method = "get_or_insert_text",  class = "TextRef"),
  list(method = "get_or_insert_map",   class = "MapRef"),
  list(method = "get_or_insert_array", class = "ArrayRef")
)) {
  local({
    test_that(paste("Doc", item$method, "returns", item$class), {
      doc <- Doc$new()
      obj <- doc[[item$method]]("root")
      expect_true(inherits(obj, item$class))
    })
  }, list(item = item))
}

test_that("Transaction$new returns a Transaction", {
  doc <- Doc$new()
  trans <- Transaction$new(doc)
  expect_true(inherits(trans, "Transaction"))
})

test_that("Text insert and retrieve get_string", {
  doc <- Doc$new()
  text <- doc$get_or_insert_text("article")

  trans <- Transaction$new(doc, mutable = TRUE)
  on.exit(trans$drop())
  text$insert(trans, 0L, "hello")
  text$insert(trans, 5L, " world")
  trans$commit()

  expect_equal(text$get_string(trans), "hello world")
  expect_equal(text$len(trans), 11L)
})

test_that("Text push appends to the end", {
  doc <- Doc$new()
  text <- doc$get_or_insert_text("article")

  trans <- Transaction$new(doc, mutable = TRUE)
  on.exit(trans$drop())
  text$push(trans, "hello")
  text$push(trans, " world")

  expect_equal(text$get_string(trans), "hello world")
})

test_that("Text remove_range removes characters", {
  doc <- Doc$new()
  text <- doc$get_or_insert_text("article")

  trans <- Transaction$new(doc, mutable = TRUE)
  on.exit(trans$drop())
  text$push(trans, "hello world")
  text$remove_range(trans, 5L, 6L)

  expect_equal(text$get_string(trans), "hello")
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

test_that("Transaction state_vector of empty doc is empty", {
  doc <- Doc$new()
  trans <- Transaction$new(doc)
  on.exit(trans$drop())
  sv <- trans$state_vector()
  expect_true(sv$is_empty())
})

test_that("Update$new creates an empty Update", {
  update <- Update$new()
  expect_true(inherits(update, "Update"))
  expect_true(update$is_empty())
})

for (version in c("v1", "v2")) {
  local({
    test_that(paste("Update encode/decode roundtrip", version), {
      update <- Update$new()
      encoded <- update[[paste0("encode_", version)]]()
      expect_true(is.raw(encoded))
      decoded <- Update[[paste0("decode_", version)]](encoded)
      expect_true(decoded$is_empty())
    })
  }, list(version = version))
}

for (version in c("v1", "v2")) {
  local({
    test_that(paste("Transaction encode_diff", version, "against current state vector returns empty update"), {
      doc <- Doc$new()
      text <- doc$get_or_insert_text("article")

      trans <- Transaction$new(doc, mutable = TRUE)
      on.exit(trans$drop())
      text$insert(trans, 0L, "hello")
      trans$commit()

      sv <- trans$state_vector()
      diff <- trans[[paste0("encode_diff_", version)]](sv)
      expect_true(is.raw(diff))
    })
  }, list(version = version))
}

for (version in c("v1", "v2")) {
  local({
    test_that(paste("Update decode", version, "errors on invalid data"), {
      expect_s3_class(Update[[paste0("decode_", version)]](as.raw(c(0xff))), "extendr_error")
    })
  }, list(version = version))
}

for (version in c("v1", "v2")) {
  local({
    test_that(paste("StateVector decode", version, "errors on invalid data"), {
      expect_s3_class(StateVector[[paste0("decode_", version)]](as.raw(c(0xff))), "extendr_error")
    })
  }, list(version = version))
}

for (version in c("v1", "v2")) {
  local({
    test_that(paste("apply_update", version, "errors on invalid data"), {
      doc <- Doc$new()
      trans <- Transaction$new(doc, mutable = TRUE)
      on.exit(trans$drop())
      expect_s3_class(trans[[paste0("apply_update_", version)]](as.raw(c(0xff))), "extendr_error")
    })
  }, list(version = version))
}

test_that("Map insert_text and contains_key", {
  doc <- Doc$new()
  map <- doc$get_or_insert_map("data")

  trans <- Transaction$new(doc, mutable = TRUE)
  on.exit(trans$drop())
  map$insert_text(trans, "key")

  expect_equal(map$len(trans), 1L)
  expect_true(map$contains_key(trans, "key"))
  expect_false(map$contains_key(trans, "other"))
})

test_that("Map remove decreases len", {
  doc <- Doc$new()
  map <- doc$get_or_insert_map("data")

  trans <- Transaction$new(doc, mutable = TRUE)
  on.exit(trans$drop())
  map$insert_text(trans, "a")
  map$insert_text(trans, "b")
  map$remove(trans, "a")

  expect_equal(map$len(trans), 1L)
  expect_false(map$contains_key(trans, "a"))
})

test_that("Map clear removes all entries", {
  doc <- Doc$new()
  map <- doc$get_or_insert_map("data")

  trans <- Transaction$new(doc, mutable = TRUE)
  on.exit(trans$drop())
  map$insert_text(trans, "a")
  map$insert_text(trans, "b")
  map$clear(trans)

  expect_equal(map$len(trans), 0L)
})

test_that("Map insert methods return usable nested types", {
  doc <- Doc$new()
  map <- doc$get_or_insert_map("data")
  trans <- Transaction$new(doc, mutable = TRUE)
  on.exit(trans$drop())

  map$insert_any(trans, "scalar", "hello")

  text <- map$insert_text(trans, "content")
  expect_true(inherits(text, "TextRef"))
  text$push(trans, "hello")
  text$push(trans, " world")
  expect_equal(text$get_string(trans), "hello world")

  arr <- map$insert_array(trans, "list")
  expect_true(inherits(arr, "ArrayRef"))
  arr$insert_any(trans, 0L, TRUE)
  expect_equal(arr$len(trans), 1L)

  nested <- map$insert_map(trans, "nested")
  expect_true(inherits(nested, "MapRef"))
  nested$insert_any(trans, "k", 42L)
  expect_equal(nested$len(trans), 1L)

  expect_equal(map$len(trans), 4L)
})

test_that("ArrayRef insert methods return usable nested types", {
  doc <- Doc$new()
  map <- doc$get_or_insert_map("data")
  trans <- Transaction$new(doc, mutable = TRUE)
  on.exit(trans$drop())
  arr <- map$insert_array(trans, "root")
  expect_true(inherits(arr, "ArrayRef"))

  arr$insert_any(trans, 0L, "hello")

  text <- arr$insert_text(trans, 1L)
  expect_true(inherits(text, "TextRef"))
  text$push(trans, "hello")
  text$push(trans, " world")
  expect_equal(text$get_string(trans), "hello world")

  nested_arr <- arr$insert_array(trans, 2L)
  expect_true(inherits(nested_arr, "ArrayRef"))
  nested_arr$insert_any(trans, 0L, 42L)
  expect_equal(nested_arr$len(trans), 1L)

  nested_map <- arr$insert_map(trans, 3L)
  expect_true(inherits(nested_map, "MapRef"))
  nested_map$insert_any(trans, "k", TRUE)
  expect_equal(nested_map$len(trans), 1L)

  expect_equal(arr$len(trans), 4L)
})

#####################
# Integration tests #
#####################

# This is the quick start example from yrs, https://docs.rs/yrs/latest/yrs/
for (version in c("v1", "v2")) {
  local({
    test_that(paste("Synchronize two docs", version), {
      doc <- Doc$new()
      text <- doc$get_or_insert_text("article")

      trans <- Transaction$new(doc, mutable = TRUE)
      text$insert(trans, 0L, "hello")
      text$insert(trans, 5L, " world")
      trans$commit()

      expect_equal(text$get_string(trans), "hello world")
      trans$drop()

      # Synchronize state with remote replica
      remote_doc <- Doc$new()
      remote_text <- remote_doc$get_or_insert_text("article")

      remote_trans <- Transaction$new(remote_doc)
      remote_sv_raw <- remote_trans$state_vector()[[paste0("encode_", version)]]()
      remote_trans$drop()

      # Get update with contents not observed by remote_doc
      local_trans <- Transaction$new(doc)
      remote_sv <- StateVector[[paste0("decode_", version)]](remote_sv_raw)
      update <- local_trans[[paste0("encode_diff_", version)]](remote_sv)
      local_trans$drop()

      # Apply update on remote doc
      remote_trans_mut <- Transaction$new(remote_doc, mutable = TRUE)
      remote_trans_mut[[paste0("apply_update_", version)]](update)
      remote_trans_mut$commit()

      expect_equal(remote_text$get_string(remote_trans_mut), "hello world")
      remote_trans_mut$drop()
    })
  }, list(version = version))
}
