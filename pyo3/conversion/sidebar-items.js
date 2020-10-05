initSidebarItems({"trait":[["AsPyPointer","This trait represents that we can do zero-cost conversion from the object to a FFI pointer."],["FromPy","Similar to [std::convert::From], just that it requires a gil token."],["FromPyObject","`FromPyObject` is implemented by various types that can be extracted from a Python object reference."],["FromPyPointer","Raw level conversion between `*mut ffi::PyObject` and PyO3 types."],["IntoPy","Similar to [std::convert::Into], just that it requires a gil token."],["IntoPyPointer","This trait allows retrieving the underlying FFI pointer from Python objects."],["PyTryFrom","Trait implemented by Python object types that allow a checked downcast. If `T` implements `PyTryFrom`, we can convert `&PyAny` to `&T`."],["PyTryInto","Trait implemented by Python object types that allow a checked downcast. This trait is similar to `std::convert::TryInto`"],["ToBorrowedObject","This trait has two implementations: The slow one is implemented for all [ToPyObject] and creates a new object using [ToPyObject::to_object], while the fast one is only implemented for AsPyPointer (we know that every AsPyPointer is also ToPyObject) and uses [AsPyPointer::as_ptr()]"],["ToPyObject","Conversion trait that allows various objects to be converted into `PyObject`."]]});