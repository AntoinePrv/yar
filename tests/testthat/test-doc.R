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

test_that("Transaction state_vector of empty doc is empty", {
  doc <- Doc$new()
  trans <- Transaction$new(doc)
  sv <- trans$state_vector()
  expect_true(sv$is_empty())
  trans$drop()
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
      text$insert(trans, 0L, "hello")
      trans$commit()

      sv <- trans$state_vector()
      diff <- trans[[paste0("encode_diff_", version)]](sv)
      expect_true(is.raw(diff))
      trans$drop()
    })
  }, list(version = version))
}

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
