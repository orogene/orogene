class FileProviderError: Error {
  internal init(error: Error) {
    self.error = error
  }

  var localizedDescription: String {
    get {
        error.localizedDescription
    }
  }

  let error: Error
}
