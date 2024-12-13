POST http://localhost:3756/completion
Content-Type: application/json
```json
{
    "prompt": "Create a new user",
    "json_schema": {
        "$schema": "http://json-schema.org/draft-07/schema#",
        "type": "array",
        "minItems": 1,
        "maxItems": 1,
        "uniqueItems": true,
        "items": {
            "oneOf": [
                {
                    "type": "object",
                    "properties": {
                        "function_name": {
                            "const": "calculate_total"
                        },
                        "items": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "name": {
                                        "type": "string"
                                    },
                                    "price": {
                                        "type": "number",
                                        "minimum": 0
                                    }
                                },
                                "required": [
                                    "name",
                                    "price"
                                ]
                            }
                        },
                        "tax_rate": {
                            "type": "number",
                            "minimum": 0,
                            "maximum": 1
                        }
                    },
                    "required": [
                        "function_name",
                        "items"
                    ]
                },
                {
                    "type": "object",
                    "properties": {
                        "function_name": {
                            "const": "send_email"
                        },
                        "to": {
                            "type": "string",
                            "format": "email"
                        },
                        "subject": {
                            "type": "string"
                        },
                        "body": {
                            "type": "string"
                        },
                        "attachments": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "filename": {
                                        "type": "string"
                                    },
                                    "content": {
                                        "type": "string"
                                    }
                                },
                                "required": [
                                    "filename",
                                    "content"
                                ]
                            }
                        }
                    },
                    "required": [
                        "function_name",
                        "to",
                        "subject",
                        "body"
                    ]
                },
                {
                    "type": "object",
                    "properties": {
                        "function_name": {
                            "const": "create_user"
                        },
                        "username": {
                            "type": "string"
                        },
                        "email": {
                            "type": "string",
                            "format": "email"
                        },
                        "password": {
                            "type": "string",
                            "minLength": 8
                        },
                        "role": {
                            "type": "string",
                            "enum": [
                                "admin",
                                "user",
                                "editor"
                            ]
                        }
                    },
                    "required": [
                        "function_name",
                        "username",
                        "email",
                        "password"
                    ]
                }
            ]
        }
    }
}
```
HTTP 200
[Asserts]
jsonpath "$.content" contains "function_name"
jsonpath "$.content" contains "create_user"