"""
Handler decorator for VIL sidecar functions.

Usage:
    @vil_handler("fraud_check")
    def fraud_check(request: dict) -> dict:
        return {"score": 0.95, "is_fraud": True}
"""

from typing import Callable, Dict, Any
import functools


def vil_handler(method_name: str):
    """
    Decorator that marks a function as a VIL sidecar handler.

    The decorated function receives a dict (deserialized from SHM JSON)
    and must return a dict (serialized back to SHM JSON).

    Args:
        method_name: The method name used in Invoke messages from the host.
    """
    def decorator(func: Callable[[Dict[str, Any]], Dict[str, Any]]):
        @functools.wraps(func)
        def wrapper(request: Dict[str, Any]) -> Dict[str, Any]:
            return func(request)
        wrapper._vil_method = method_name
        return wrapper
    return decorator
