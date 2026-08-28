# ADR 0006: Capability ABI

Status: Accepted

External effects are namespaced, versioned Rust capabilities explicitly listed
in the app manifest. Inputs and outputs use schema-checked `UiValue`; expensive
native resources use registered opaque handles.

Capability calls are forbidden during `view`. M1 Task and Subscription handles
extend this ABI without exposing arbitrary Rust objects.
