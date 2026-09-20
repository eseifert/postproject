"""Structured exceptions translated from stable C ABI error codes."""


class PostProjectError(RuntimeError):
    """Base error reported by the native PostProject library."""

    def __init__(self, code: int, message: str) -> None:
        super().__init__(message)
        self.code = code


class InvalidArgumentError(PostProjectError, ValueError):
    """An argument violates the public domain contract."""


class NotFoundError(PostProjectError):
    """A requested project object does not exist."""


class AlreadyExistsError(PostProjectError):
    """A unique project object or attachment already exists."""


class StorageError(PostProjectError):
    """Persistent project data could not be read or written safely."""


class ConflictError(PostProjectError):
    """An operation conflicts with current transaction or project state."""


class UnsupportedError(PostProjectError):
    """The requested operation is not supported by the current ABI."""


ERROR_TYPES: dict[int, type[PostProjectError]] = {
    1: InvalidArgumentError,
    2: NotFoundError,
    3: AlreadyExistsError,
    5: StorageError,
    7: ConflictError,
    10: UnsupportedError,
}

