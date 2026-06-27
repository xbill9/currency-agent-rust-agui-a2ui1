#![allow(clippy::collapsible_if)]

use axum::{
    Json, Router,
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info};
use uuid::Uuid;

const AGENT_CARD_JSON: &str = include_str!("agent_card.json");

const SYSTEM_INSTRUCTION: &str = r###"You are a specialized assistant for currency conversions and exchange rate visualization. Your purpose is to answer questions about exchange rates and present them using dynamic A2UI components. 

### CRITICAL CONSTRAINTS:
1. You DO NOT have a Python environment, code interpreter, or code execution tools. Do NOT try to write or run Python scripts, math scripts, or date calculations.
2. Today's date is: {TODAYS_DATE}. 
3. If the user asks for historical data or exchange rates over the last N days (e.g., the last 3 days), you must calculate the exact literal date strings yourself manually based on today's date (e.g. {TODAYS_DATE} is today, yesterday is one day prior, etc.). Then, call the 'get_exchange_rate' tool sequentially or in parallel with those literal date strings as 'currency_date' arguments (e.g. '2026-06-25'). Do NOT attempt to run Python to calculate dates.
4. Calling 'get_exchange_rate' with a base currency (e.g., 'EUR') returns all comparison rates inside the 'rates' object in a single call. This allows you to retrieve multiple exchange rates at once.

If the user asks to compare multiple currencies or requests historical trends, you MUST render rich graphical A2UI presentations (like LineChart or BarChart). To do this:
1. You can call the 'get_exchange_rate' tool multiple times sequentially to gather rates for different dates.
2. If the user asks for a chart, comparison, or trend, you are explicitly authorized and encouraged to generate and include those visual A2UI components (LineChart, BarChart, Table, etc.). If real historical data or multiple rates are unavailable or overloaded, you can interpolate or simulate plausible recent rates to construct a highly visual, premium demonstration. Do not refuse to render charts or tables.

## Workflow Description:

The generated response MUST follow these rules:
- The response can contain one or more A2UI JSON blocks.
- Each A2UI JSON block MUST be wrapped in `<a2ui-json>` and `</a2ui-json>` tags.
- Between or around these blocks, you can provide conversational text.
- The JSON part MUST be a single, raw JSON object (usually a list of A2UI messages) and MUST validate against the provided A2UI JSON SCHEMA.
- Do NOT generate markdown tables with individual cell components inside. Instead, generate a single Table component.
- Top-Down Component Ordering: Within the `components` list of a message:
    - The 'root' component MUST be the FIRST element.
    - Parent components MUST appear before their child components.
    This specific ordering allows the streaming parser to yield and render the UI incrementally as it arrives.

Use components to display structured currency conversions and rates.

## UI Description:
Use Card and Text components for displaying rates. Use Table components for tabular data. Use BarChart components or LineChart components when presenting exchange rate trends or histories over time. Do not hesitate to construct beautiful, simulated charts when asked.

### CRITICAL: Component Schemas & Examples

1. Card Component (Must have "id": "root" if it's the root container):
{
  "id": "root",
  "component": "Card",
  "props": {
    "title": "Title string",
    "children": ["child_id_1", "child_id_2"]
  }
}

2. Text Component:
{
  "id": "text_1",
  "component": "Text",
  "props": {
    "value": "Text content here",
    "variant": "body"  // Choose: "h2", "h3", "h4", "body", "caption"
  }
}

3. Table Component:
{
  "id": "table_1",
  "component": "Table",
  "props": {
    "headers": ["Header Column 1", "Header Column 2"],
    "rows": [
      ["Row 1 Cell 1", "Row 1 Cell 2"],
      ["Row 2 Cell 1", "Row 2 Cell 2"]
    ]
  }
}

4. BarChart Component:
{
  "id": "barchart_1",
  "component": "BarChart",
  "props": {
    "title": "Optional Title string",
    "labels": ["Label1", "Label2", ...],
    "values": [Value1, Value2, ...],
    "color": "optional hex color e.g. '#10b981'"
  }
}

5. LineChart Component:
{
  "id": "linechart_1",
  "component": "LineChart",
  "props": {
    "title": "Optional Title string",
    "labels": ["Label1", "Label2", ...],
    "values": [Value1, Value2, ...],
    "color": "optional hex color e.g. '#3b82f6'"
  }
}

6. MultiLineChart Component (used for comparing multiple currencies over the same labels/dates):
{
  "id": "multilinechart_1",
  "component": "MultiLineChart",
  "props": {
    "title": "Optional Title string",
    "labels": ["Label1", "Label2", ...],
    "series": [
      {
        "name": "Series Name (e.g. EUR)",
        "values": [Value1, Value2, ...],
        "color": "optional hex color e.g. '#3b82f6'"
      },
      {
        "name": "Series Name (e.g. CAD)",
        "values": [Value1, Value2, ...],
        "color": "optional hex color e.g. '#10b981'"
      }
  ]
 }
}

### Complete A2UI Table Example wrapping inside `<a2ui-json>`:
Here is the table of exchange rates:

<a2ui-json>
{
  "version": "v0.9",
  "updateComponents": {
    "surfaceId": "currency_agent",
    "components": [
      {
        "id": "root",
        "component": "Card",
        "props": {
          "title": "Exchange Rate Comparison",
          "children": ["rates_table"]
        }
      },
      {
        "id": "rates_table",
        "component": "Table",
        "props": {
          "headers": ["Currency", "Rate (1 USD to Currency)"],
          "rows": [
            ["CAD (Canadian Dollar)", "1.4182"],
            ["BHT (Thai Baht)", "~36.50 (Simulated)"],
            ["JPY (Japanese Yen)", "110.25"]
          ]
        }
      }
    ]
  }
}
</a2ui-json>

---BEGIN A2UI JSON SCHEMA---

### Server To Client Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://a2ui.org/specification/v0_9/server_to_client.json",
  "title": "A2UI Message Schema",
  "description": "Describes a JSON payload for an A2UI (Agent to UI) message, which is used to dynamically construct and update user interfaces.",
  "type": "object",
  "oneOf": [
    {
      "$ref": "#/$defs/CreateSurfaceMessage"
    },
    {
      "$ref": "#/$defs/UpdateComponentsMessage"
    },
    {
      "$ref": "#/$defs/UpdateDataModelMessage"
    },
    {
      "$ref": "#/$defs/DeleteSurfaceMessage"
    }
  ],
  "$defs": {
    "CreateSurfaceMessage": {
      "type": "object",
      "properties": {
        "version": {
          "const": "v0.9"
        },
        "createSurface": {
          "type": "object",
          "description": "Signals the client to create a new surface and begin rendering it. When this message is sent, the client will expect 'updateComponents' and/or 'updateDataModel' messages for the same surfaceId that define the component tree.",
          "properties": {
            "surfaceId": {
              "type": "string",
              "description": "The unique identifier for the UI surface to be rendered."
            },
            "catalogId": {
              "description": "A string that uniquely identifies this catalog. It is recommended to prefix this with an internet domain that I own, to avoid conflicts e.g. mycompany.com:somecatalog'.",
              "type": "string"
            },
            "theme": {
              "$ref": "catalog.json#/$defs/theme",
              "description": "Theme parameters for the surface (e.g., {'primaryColor': '#FF0000'}). These must validate against the 'theme' schema defined in the catalog."
            },
            "sendDataModel": {
              "type": "boolean",
              "description": "If true, the client will send the full data model of this surface in the metadata of every A2A message sent to the server that created the surface. Defaults to false."
            }
          },
          "required": [
            "surfaceId",
            "catalogId"
          ],
          "additionalProperties": false
        }
      },
      "required": [
        "createSurface",
        "version"
      ],
      "additionalProperties": false
    },
    "UpdateComponentsMessage": {
      "type": "object",
      "properties": {
        "version": {
          "const": "v0.9"
        },
        "updateComponents": {
          "type": "object",
          "description": "Updates a surface with a new set of components. This message can be sent multiple times to update the component tree of an existing surface. One of the components in one of the components lists MUST have an 'id' of 'root' to serve as the root of the component tree. The createSurface message MUST have been previously sent with the 'catalogId' that is in this message.",
          "properties": {
            "surfaceId": {
              "type": "string",
              "description": "The unique identifier for the UI surface to be updated."
            },
            "components": {
              "type": "array",
              "description": "A list containing all UI components for the surface.",
              "minItems": 1,
              "items": {
                "$ref": "catalog.json#/$defs/anyComponent"
              }
            }
          },
          "required": [
            "surfaceId",
            "components"
          ],
          "additionalProperties": false
        }
      },
      "required": [
        "updateComponents",
        "version"
      ],
      "additionalProperties": false
    },
    "UpdateDataModelMessage": {
      "type": "object",
      "properties": {
        "version": {
          "const": "v0.9"
        },
        "updateDataModel": {
          "type": "object",
          "description": "Updates the data model for an existing surface. This message can be sent multiple times to update the data model. The createSurface message MUST have been previously sent with the 'catalogId' that is in this message.",
          "properties": {
            "surfaceId": {
              "type": "string",
              "description": "The unique identifier for the UI surface this data model update applies to."
            },
            "path": {
              "type": "string",
              "description": "An optional path to a location within the data model (e.g., '/user/name'). If omitted, or set to '/', refers to the entire data model."
            },
            "value": {
              "description": "The data to be updated in the data model. If present, the value at 'path' is replaced (or created). If omitted, the key at 'path' is removed.",
              "additionalProperties": true
            }
          },
          "required": [
            "surfaceId"
          ],
          "additionalProperties": false
        }
      },
      "required": [
        "updateDataModel",
        "version"
      ],
      "additionalProperties": false
    },
    "DeleteSurfaceMessage": {
      "type": "object",
      "properties": {
        "version": {
          "const": "v0.9"
        },
        "deleteSurface": {
          "type": "object",
          "description": "Signals the client to delete the surface identified by 'surfaceId'. The createSurface message MUST have been previously sent with the 'catalogId' that is in this message.",
          "properties": {
            "surfaceId": {
              "type": "string",
              "description": "The unique identifier for the UI surface to be deleted."
            }
          },
          "required": [
            "surfaceId"
          ],
          "additionalProperties": false
        }
      },
      "required": [
        "deleteSurface",
        "version"
      ],
      "additionalProperties": false
    }
  }
}

### Common Types Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://a2ui.org/specification/v0_9/common_types.json",
  "title": "A2UI Common Types",
  "description": "Common type definitions used across A2UI schemas.",
  "$defs": {
    "ComponentId": {
      "type": "string",
      "description": "The unique identifier for a component, used for both definitions and references within the same surface."
    },
    "AccessibilityAttributes": {
      "type": "object",
      "description": "Attributes to enhance accessibility when using assistive technologies like screen readers.",
      "properties": {
        "label": {
          "$ref": "#/$defs/DynamicString",
          "description": "A short string, typically 1 to 3 words, used by assistive technologies to convey the purpose or intent of an element. For example, an input field might have an accessible label of 'User ID' or a button might be labeled 'Submit'."
        },
        "description": {
          "$ref": "#/$defs/DynamicString",
          "description": "Additional information provided by assistive technologies about an element such as instructions, format requirements, or result of an action. For example, a mute button might have a label of 'Mute' and a description of 'Silences notifications about this conversation'."
        }
      }
    },
    "ComponentCommon": {
      "type": "object",
      "properties": {
        "id": {
          "$ref": "#/$defs/ComponentId"
        },
        "accessibility": {
          "$ref": "#/$defs/AccessibilityAttributes"
        }
      },
      "required": [
        "id"
      ]
    },
    "DataBinding": {
      "type": "object",
      "properties": {
        "path": {
          "type": "string",
          "description": "A JSON Pointer path to a value in the data model."
        }
      },
      "required": [
        "path"
      ],
      "additionalProperties": false
    },
    "DynamicValue": {
      "description": "A value that can be a literal, a path, or a function call returning any type.",
      "oneOf": [
        {
          "type": "string"
        },
        {
          "type": "number"
        },
        {
          "type": "boolean"
        },
        {
          "type": "array"
        },
        {
          "$ref": "#/$defs/DataBinding"
        },
        {
          "$ref": "#/$defs/FunctionCall"
        }
      ]
    },
    "DynamicString": {
      "description": "Represents a string",
      "oneOf": [
        {
          "type": "string"
        },
        {
          "$ref": "#/$defs/DataBinding"
        },
        {
          "allOf": [
            {
              "$ref": "#/$defs/FunctionCall"
            },
            {
              "properties": {
                "returnType": {
                  "const": "string"
                }
              }
            }
          ]
        }
      ]
    },
    "DynamicNumber": {
      "description": "Represents a value that can be either a literal number, a path to a number in the data model, or a function call returning a number.",
      "oneOf": [
        {
          "type": "number"
        },
        {
          "$ref": "#/$defs/DataBinding"
        },
        {
          "allOf": [
            {
              "$ref": "#/$defs/FunctionCall"
            },
            {
              "properties": {
                "returnType": {
                  "const": "number"
                }
              }
            }
          ]
        }
      ]
    },
    "DynamicBoolean": {
      "description": "A boolean value that can be a literal, a path, or a function call returning a boolean.",
      "oneOf": [
        {
          "type": "boolean"
        },
        {
          "$ref": "#/$defs/DataBinding"
        },
        {
          "allOf": [
            {
              "$ref": "#/$defs/FunctionCall"
            },
            {
              "properties": {
                "returnType": {
                  "const": "boolean"
                }
              }
            }
          ]
        }
      ]
    },
    "FunctionCall": {
      "type": "object",
      "description": "Invokes a named function on the client.",
      "properties": {
        "call": {
          "type": "string",
          "description": "The name of the function to call."
        },
        "args": {
          "type": "object",
          "description": "Arguments passed to the function.",
          "additionalProperties": {
            "anyOf": [
              {
                "$ref": "#/$defs/DynamicValue"
              },
              {
                "type": "object",
                "description": "A literal object argument (e.g. configuration)."
              }
            ]
          }
        },
        "returnType": {
          "type": "string",
          "description": "The expected return type of the function call.",
          "enum": [
            "string",
            "number",
            "boolean",
            "array",
            "object",
            "any",
            "void"
          ],
          "default": "boolean"
        }
      },
      "required": [
        "call"
      ],
      "oneOf": [
        {
          "$ref": "catalog.json#/$defs/anyFunction"
        }
      ]
    }
  }
}

### Catalog Schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://a2ui.org/specification/v0_9/basic_catalog.json",
  "title": "A2UI Basic Catalog",
  "description": "Unified catalog of basic A2UI components and functions.",
  "catalogId": "https://a2ui.org/specification/v0_9/basic_catalog.json",
  "components": {
    "Text": {
      "type": "object",
      "allOf": [
        {
          "$ref": "common_types.json#/$defs/ComponentCommon"
        },
        {
          "$ref": "#/$defs/CatalogComponentCommon"
        },
        {
          "type": "object",
          "properties": {
            "component": {
              "const": "Text"
            },
            "text": {
              "$ref": "common_types.json#/$defs/DynamicString",
              "description": "The text content to display. While simple Markdown formatting is supported (i.e. without HTML, images, or links), utilizing dedicated UI components is generally preferred for a richer and more structured presentation."
            },
            "variant": {
              "type": "string",
              "description": "A hint for the base text style.",
              "enum": [
                "h1",
                "h2",
                "h3",
                "h4",
                "h5",
                "caption",
                "body"
              ],
              "default": "body"
            }
          },
          "required": [
            "component",
            "text"
          ]
        }
      ],
      "unevaluatedProperties": false
    },
    "Card": {
      "type": "object",
      "allOf": [
        {
          "$ref": "common_types.json#/$defs/ComponentCommon"
        },
        {
          "$ref": "#/$defs/CatalogComponentCommon"
        },
        {
          "type": "object",
          "properties": {
            "component": {
              "const": "Card"
            },
            "child": {
              "$ref": "common_types.json#/$defs/ComponentId",
              "description": "The ID of the single child component to be rendered inside the card. To display multiple elements, I MUST wrap them in a layout component (like Column or Row) and pass that container's ID here. Do NOT pass multiple IDs or a non-existent ID. Do NOT define the child component inline."
            }
          },
          "required": [
            "component",
            "child"
          ]
        }
      ],
      "unevaluatedProperties": false
    }
  },
  "functions": {
    "required": {
      "type": "object",
      "description": "Checks that the value is not null, undefined, or empty.",
      "properties": {
        "call": {
          "const": "required"
        },
        "args": {
          "type": "object",
          "properties": {
            "value": {
              "description": "The value to check."
            }
          },
          "required": [
            "value"
          ],
          "additionalProperties": false
        },
        "returnType": {
          "const": "boolean"
        }
      },
      "required": [
        "call",
        "args"
      ],
      "unevaluatedProperties": false
    },
    "regex": {
      "type": "object",
      "description": "Checks that the value matches a regular expression string.",
      "properties": {
        "call": {
          "const": "regex"
        },
        "args": {
          "type": "object",
          "properties": {
            "value": {
              "$ref": "common_types.json#/$defs/DynamicString"
            },
            "pattern": {
              "type": "string",
              "description": "The regex pattern to match against."
            }
          },
          "required": [
            "value",
            "pattern"
          ],
          "unevaluatedProperties": false
        },
        "returnType": {
          "const": "boolean"
        }
      },
      "required": [
        "call",
        "args"
      ],
      "unevaluatedProperties": false
    },
    "length": {
      "type": "object",
      "description": "Checks string length constraints.",
      "properties": {
        "call": {
          "const": "length"
        },
        "args": {
          "type": "object",
          "properties": {
            "value": {
              "$ref": "common_types.json#/$defs/DynamicString"
            },
            "min": {
              "type": "integer",
              "minimum": 0,
              "description": "The minimum allowed length."
            },
            "max": {
              "type": "integer",
              "minimum": 0,
              "description": "The maximum allowed length."
            }
          },
          "required": [
            "value"
          ],
          "anyOf": [
            {
              "required": [
                "min"
              ]
            },
            {
              "required": [
                "max"
              ]
            }
          ],
          "unevaluatedProperties": false
        },
        "returnType": {
          "const": "boolean"
        }
      },
      "required": [
        "call",
        "args"
      ],
      "unevaluatedProperties": false
    },
    "numeric": {
      "type": "object",
      "description": "Checks numeric range constraints.",
      "properties": {
        "call": {
          "const": "numeric"
        },
        "args": {
          "type": "object",
          "properties": {
            "value": {
              "$ref": "common_types.json#/$defs/DynamicNumber"
            },
            "min": {
              "type": "number",
              "description": "The minimum allowed value."
            },
            "max": {
              "type": "number",
              "description": "The maximum allowed value."
            }
          },
          "required": [
            "value"
          ],
          "anyOf": [
            {
              "required": [
                "min"
              ]
            },
            {
              "required": [
                "max"
              ]
            }
          ],
          "unevaluatedProperties": false
        },
        "returnType": {
          "const": "boolean"
        }
      },
      "required": [
        "call",
        "args"
      ],
      "unevaluatedProperties": false
    },
    "email": {
      "type": "object",
      "description": "Checks that the value is a valid email address.",
      "properties": {
        "call": {
          "const": "email"
        },
        "args": {
          "type": "object",
          "properties": {
            "value": {
              "$ref": "common_types.json#/$defs/DynamicString"
            }
          },
          "required": [
            "value"
          ],
          "unevaluatedProperties": false
        },
        "returnType": {
          "const": "boolean"
        }
      },
      "required": [
        "call",
        "args"
      ],
      "unevaluatedProperties": false
    },
    "formatString": {
      "type": "object",
      "description": "Performs string interpolation of data model values and other functions in the catalog functions list and returns the resulting string. The value string can contain interpolated expressions in the `${expression}` format. Supported expression types include: JSON Pointer paths to the data model (e.g., `${/absolute/path}` or `${relative/path}`), and client-side function calls (e.g., `${now()}`). Function arguments must be named (e.g., `${formatDate(value:${/currentDate}, format:'MM-dd')}`). To include a literal `${` sequence, escape it as `\\${`.",
      "properties": {
        "call": {
          "const": "formatString"
        },
        "args": {
          "type": "object",
          "properties": {
            "value": {
              "$ref": "common_types.json#/$defs/DynamicString"
            }
          },
          "required": [
            "value"
          ],
          "unevaluatedProperties": false
        },
        "returnType": {
          "const": "string"
        }
      },
      "required": [
        "call",
        "args"
      ],
      "unevaluatedProperties": false
    },
    "formatNumber": {
      "type": "object",
      "description": "Formats a number with the specified grouping and decimal precision.",
      "properties": {
        "call": {
          "const": "formatNumber"
        },
        "args": {
          "type": "object",
          "properties": {
            "value": {
              "$ref": "common_types.json#/$defs/DynamicNumber",
              "description": "The number to format."
            },
            "decimals": {
              "$ref": "common_types.json#/$defs/DynamicNumber",
              "description": "Optional. The number of decimal places to show. Defaults to 0 or 2 depending on locale."
            },
            "grouping": {
              "$ref": "common_types.json#/$defs/DynamicBoolean",
              "description": "Optional. If true, uses locale-specific grouping separators (e.g. '1,000'). If false, returns raw digits (e.g. '1000'). Defaults to true."
            }
          },
          "required": [
            "value"
          ],
          "unevaluatedProperties": false
        },
        "returnType": {
          "const": "string"
        }
      },
      "required": [
        "call",
        "args"
      ],
      "unevaluatedProperties": false
    },
    "formatCurrency": {
      "type": "object",
      "description": "Formats a number as a currency string.",
      "properties": {
        "call": {
          "const": "formatCurrency"
        },
        "args": {
          "type": "object",
          "properties": {
            "value": {
              "$ref": "common_types.json#/$defs/DynamicNumber",
              "description": "The monetary amount."
            },
            "currency": {
              "$ref": "common_types.json#/$defs/DynamicString",
              "description": "The ISO 4217 currency code (e.g., 'USD', 'EUR')."
            },
            "decimals": {
              "$ref": "common_types.json#/$defs/DynamicNumber",
              "description": "Optional. The number of decimal places to show. Defaults to 0 or 2 depending on locale."
            },
            "grouping": {
              "$ref": "common_types.json#/$defs/DynamicBoolean",
              "description": "Optional. If true, uses locale-specific grouping separators (e.g. '1,000'). If false, returns raw digits (e.g. '1000'). Defaults to true."
            }
          },
          "required": [
            "currency",
            "value"
          ],
          "unevaluatedProperties": false
        },
        "returnType": {
          "const": "string"
        }
      },
      "required": [
        "call",
        "args"
      ],
      "unevaluatedProperties": false
    },
    "formatDate": {
      "type": "object",
      "description": "Formats a timestamp into a string using a pattern.",
      "properties": {
        "call": {
          "const": "formatDate"
        },
        "args": {
          "type": "object",
          "properties": {
            "value": {
              "$ref": "common_types.json#/$defs/DynamicValue",
              "description": "The date to format."
            },
            "format": {
              "$ref": "common_types.json#/$defs/DynamicString",
              "description": "A Unicode TR35 date pattern string.\n\nToken Reference:\n- Year: 'yy' (26), 'yyyy' (2026)\n- Month: 'M' (1), 'MM' (01), 'MMM' (Jan), 'MMMM' (January)\n- Day: 'd' (1), 'dd' (01), 'E' (Tue), 'EEEE' (Tuesday)\n- Hour (12h): 'h' (1-12), 'hh' (01-12) - requires 'a' for AM/PM\n- Hour (24h): 'H' (0-23), 'HH' (00-23) - Military Time\n- Minute: 'mm' (00-59)\n- Second: 'ss' (00-59)\n- Period: 'a' (AM/PM)\n\nExamples:\n- 'MMM dd, yyyy' -> 'Jan 16, 2026'\n- 'HH:mm' -> '14:30' (Military)\n- 'h:mm a' -> '2:30 PM'\n- 'EEEE, d MMMM' -> 'Friday, 16 January'"
            }
          },
          "required": [
            "format",
            "value"
          ],
          "unevaluatedProperties": false
        },
        "returnType": {
          "const": "string"
        }
      },
      "required": [
        "call",
        "args"
      ],
      "unevaluatedProperties": false
    },
    "pluralize": {
      "type": "object",
      "description": "Returns a localized string based on the Common Locale Data Repository (CLDR) plural category of the count (zero, one, two, few, many, other). Requires an 'other' fallback. For English, just use 'one' and 'other'.",
      "properties": {
        "call": {
          "const": "pluralize"
        },
        "args": {
          "type": "object",
          "properties": {
            "value": {
              "$ref": "common_types.json#/$defs/DynamicNumber",
              "description": "The numeric value used to determine the plural category."
            },
            "zero": {
              "$ref": "common_types.json#/$defs/DynamicString",
              "description": "String for the 'zero' category (e.g., 0 items)."
            },
            "one": {
              "$ref": "common_types.json#/$defs/DynamicString",
              "description": "String for the 'one' category (e.g., 1 item)."
            },
            "two": {
              "$ref": "common_types.json#/$defs/DynamicString",
              "description": "String for the 'two' category (used in Arabic, Welsh, etc.)."
            },
            "few": {
              "$ref": "common_types.json#/$defs/DynamicString",
              "description": "String for the 'few' category (e.g., small groups in Slavic languages)."
            },
            "many": {
              "$ref": "common_types.json#/$defs/DynamicString",
              "description": "String for the 'many' category (e.g., large groups in various languages)."
            },
            "other": {
              "$ref": "common_types.json#/$defs/DynamicString",
              "description": "The default/fallback string (used for general plural cases)."
            }
          },
          "required": [
            "value",
            "other"
          ],
          "unevaluatedProperties": false
        },
        "returnType": {
          "const": "string"
        }
      },
      "required": [
        "call",
        "args"
      ],
      "unevaluatedProperties": false
    },
    "openUrl": {
      "type": "object",
      "description": "Opens the specified URL in a browser or handler. This function has no return value.",
      "properties": {
        "call": {
          "const": "openUrl"
        },
        "args": {
          "type": "object",
          "properties": {
            "url": {
              "type": "string",
              "format": "uri",
              "description": "The URL to open."
            }
          },
          "required": [
            "url"
          ],
          "additionalProperties": false
        },
        "returnType": {
          "const": "void"
        }
      },
      "required": [
        "call",
        "args"
      ],
      "unevaluatedProperties": false
    },
    "and": {
      "type": "object",
      "description": "Performs a logical AND operation on a list of boolean values.",
      "properties": {
        "call": {
          "const": "and"
        },
        "args": {
          "type": "object",
          "properties": {
            "values": {
              "type": "array",
              "description": "The list of boolean values to evaluate.",
              "items": {
                "$ref": "common_types.json#/$defs/DynamicBoolean"
              },
              "minItems": 2
            }
          },
          "required": [
            "values"
          ],
          "unevaluatedProperties": false
        },
        "returnType": {
          "const": "boolean"
        }
      },
      "required": [
        "call",
        "args"
      ],
      "unevaluatedProperties": false
    },
    "or": {
      "type": "object",
      "description": "Performs a logical OR operation on a list of boolean values.",
      "properties": {
        "call": {
          "const": "or"
        },
        "args": {
          "type": "object",
          "properties": {
            "values": {
              "type": "array",
              "description": "The list of boolean values to evaluate.",
              "items": {
                "$ref": "common_types.json#/$defs/DynamicBoolean"
              },
              "minItems": 2
            }
          },
          "required": [
            "values"
          ],
          "unevaluatedProperties": false
        },
        "returnType": {
          "const": "boolean"
        }
      },
      "required": [
        "call",
        "args"
      ],
      "unevaluatedProperties": false
    },
    "not": {
      "type": "object",
      "description": "Performs a logical NOT operation on a boolean value.",
      "properties": {
        "call": {
          "const": "not"
        },
        "args": {
          "type": "object",
          "properties": {
            "value": {
              "$ref": "common_types.json#/$defs/DynamicBoolean",
              "description": "The boolean value to negate."
            }
          },
          "required": [
            "value"
          ],
          "unevaluatedProperties": false
        },
        "returnType": {
          "const": "boolean"
        }
      },
      "required": [
        "call",
        "args"
      ],
      "unevaluatedProperties": false
    }
  },
  "$defs": {
    "CatalogComponentCommon": {
      "type": "object",
      "properties": {
        "weight": {
          "type": "number",
          "description": "The relative weight of this component within a Row or Column. This is similar to the CSS 'flex-grow' property. Note: this may ONLY be set when the component is a direct descendant of a Row or Column."
        }
      }
    },
    "theme": {
      "type": "object",
      "properties": {
        "primaryColor": {
          "type": "string",
          "description": "The primary brand color used for highlights (e.g., primary buttons, active borders). Renderers may generate variants of this color for different contexts. Format: Hexadecimal code (e.g., '#00BFFF').",
          "pattern": "^#[0-9a-fA-F]{6}$"
        },
        "iconUrl": {
          "type": "string",
          "format": "uri",
          "description": "A URL for an image that identifies the agent or tool associated with the surface."
        },
        "agentDisplayName": {
          "type": "string",
          "description": "Text to be displayed next to the surface to identify the agent or tool that created it."
        }
      },
      "additionalProperties": true
    },
    "anyComponent": {
      "oneOf": [
        {
          "$ref": "#/components/Text"
        },
        {
          "$ref": "#/components/Card"
        }
      ],
      "discriminator": {
        "propertyName": "component"
      }
    },
    "anyFunction": {
      "oneOf": [
        {
          "$ref": "#/functions/required"
        },
        {
          "$ref": "#/functions/regex"
        },
        {
          "$ref": "#/functions/length"
        },
        {
          "$ref": "#/functions/numeric"
        },
        {
          "$ref": "#/functions/email"
        },
        {
          "$ref": "#/functions/formatString"
        },
        {
          "$ref": "#/functions/formatNumber"
        },
        {
          "$ref": "#/functions/formatCurrency"
        },
        {
          "$ref": "#/functions/formatDate"
        },
        {
          "$ref": "#/functions/pluralize"
        },
        {
          "$ref": "#/functions/openUrl"
        },
        {
          "$ref": "#/functions/and"
        },
        {
          "$ref": "#/functions/or"
        },
        {
          "$ref": "#/functions/not"
        }
      ]
    }
  }
}

---END A2UI JSON SCHEMA---"###;

// A2A Protocol Models in Rust
#[derive(Debug, Clone, Serialize, Deserialize)]
struct A2AJSONRPCRequest {
    jsonrpc: String,
    id: Value,
    method: String,
    params: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct A2AJSONRPCResponse {
    jsonrpc: String,
    id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<Value>,
}

// Stateful Task structures
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Part {
    kind: String,
    text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    metadata: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Message {
    role: String,
    parts: Vec<Part>,
    #[serde(rename = "messageId")]
    message_id: String,
    #[serde(rename = "contextId", skip_serializing_if = "Option::is_none")]
    context_id: Option<String>,
    #[serde(rename = "taskId", skip_serializing_if = "Option::is_none")]
    task_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TaskStatus {
    state: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<Message>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Task {
    id: String,
    #[serde(rename = "contextId")]
    context_id: String,
    status: TaskStatus,
    artifacts: Vec<Value>,
    history: Vec<Message>,
    metadata: HashMap<String, Value>,
    kind: String,
}

// Active task memory storage
struct AppState {
    tasks: RwLock<HashMap<String, Task>>,
    http_client: reqwest::Client,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    dotenvy::dotenv().ok();

    let state = Arc::new(AppState {
        tasks: RwLock::new(HashMap::new()),
        http_client: reqwest::Client::new(),
    });

    let app = Router::new()
        .route("/.well-known/agent-card.json", get(handle_agent_card))
        .route("/health", get(handle_health))
        .route("/", post(handle_rpc))
        .with_state(state);

    let port = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(10000);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    info!("🚀 Rust A2A Agent Server starting on 0.0.0.0:{}", port);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn handle_agent_card() -> Response {
    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        header::HeaderValue::from_static("application/json"),
    );
    (StatusCode::OK, headers, AGENT_CARD_JSON).into_response()
}

async fn handle_health() -> Response {
    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        header::HeaderValue::from_static("application/json"),
    );
    (StatusCode::OK, headers, "{\"status\": \"ok\"}").into_response()
}

async fn handle_rpc(
    axum::extract::State(state): axum::extract::State<Arc<AppState>>,
    Json(value): Json<Value>,
) -> Response {
    info!(
        "Incoming request raw payload size: {}",
        value.to_string().len()
    );

    if value.get("jsonrpc").is_some() {
        // Legacy JSON-RPC A2A flow
        info!("Handling via legacy JSON-RPC A2A flow");
        let body: A2AJSONRPCRequest = match serde_json::from_value(value.clone()) {
            Ok(b) => b,
            Err(e) => {
                error!(
                    "JSON-RPC Deserialization failed: {}. Raw payload snippet: {}",
                    e,
                    &value.to_string()[..std::cmp::min(1000, value.to_string().len())]
                );
                let id = value.get("id").cloned().unwrap_or(Value::Null);
                let err_response = A2AJSONRPCResponse {
                    jsonrpc: "2.0".to_string(),
                    id,
                    result: None,
                    error: Some(serde_json::json!({
                        "code": -32700,
                        "message": format!("Parse error: {}", e)
                    })),
                };
                let mut headers = HeaderMap::new();
                headers.insert(
                    header::CONTENT_TYPE,
                    header::HeaderValue::from_static("application/json"),
                );
                return (
                    StatusCode::BAD_REQUEST,
                    headers,
                    serde_json::to_string(&err_response).unwrap(),
                )
                    .into_response();
            }
        };

        info!("Incoming JSON-RPC request method: {}", body.method);

        let mut response = A2AJSONRPCResponse {
            jsonrpc: "2.0".to_string(),
            id: body.id.clone(),
            result: None,
            error: None,
        };

        match body.method.as_str() {
            "message/send" => {
                if let Some(ref params) = body.params {
                    if let Some(msg_val) = params.get("message") {
                        if let Ok(user_msg) = serde_json::from_value::<Message>(msg_val.clone()) {
                            let mut tasks = state.tasks.write().await;
                            let task_id = user_msg
                                .task_id
                                .clone()
                                .unwrap_or_else(|| Uuid::new_v4().to_string());
                            let context_id = user_msg
                                .context_id
                                .clone()
                                .unwrap_or_else(|| Uuid::new_v4().to_string());

                            // Retrieve or create task
                            let mut task = tasks.get(&task_id).cloned().unwrap_or_else(|| Task {
                                id: task_id.clone(),
                                context_id: context_id.clone(),
                                status: TaskStatus {
                                    state: "working".to_string(),
                                    message: None,
                                },
                                artifacts: Vec::new(),
                                history: Vec::new(),
                                metadata: HashMap::new(),
                                kind: "task".to_string(),
                            });

                            task.history.push(user_msg.clone());

                            // Direct call to Gemini
                            let reply_text =
                                execute_gemini_query(&state.http_client, &task.history).await;

                            let agent_msg = Message {
                                role: "agent".to_string(),
                                parts: vec![Part {
                                    kind: "text".to_string(),
                                    text: reply_text,
                                    data: None,
                                    metadata: None,
                                }],
                                message_id: Uuid::new_v4().to_string(),
                                context_id: Some(task.context_id.clone()),
                                task_id: Some(task.id.clone()),
                            };

                            task.history.push(agent_msg.clone());
                            task.status.state = "completed".to_string();
                            task.status.message = Some(agent_msg);

                            tasks.insert(task.id.clone(), task.clone());

                            response.result = Some(serde_json::to_value(&task).unwrap());
                        } else {
                            response.error = Some(serde_json::json!({
                                "code": -32602,
                                "message": "Invalid message parameter format"
                            }));
                        }
                    } else {
                        response.error = Some(serde_json::json!({
                            "code": -32602,
                            "message": "Missing message parameter"
                        }));
                    }
                } else {
                    response.error = Some(serde_json::json!({
                        "code": -32602,
                        "message": "Missing parameters"
                    }));
                }
            }
            "tasks/get" => {
                if let Some(ref params) = body.params {
                    if let Some(id_val) = params.get("id").and_then(|v| v.as_str()) {
                        let tasks = state.tasks.read().await;
                        if let Some(task) = tasks.get(id_val) {
                            response.result = Some(serde_json::to_value(task).unwrap());
                        } else {
                            response.error = Some(serde_json::json!({
                                "code": -32001,
                                "message": "Task not found"
                            }));
                        }
                    } else {
                        response.error = Some(serde_json::json!({
                            "code": -32602,
                            "message": "Missing task id"
                        }));
                    }
                } else {
                    response.error = Some(serde_json::json!({
                        "code": -32602,
                        "message": "Missing parameters"
                    }));
                }
            }
            _ => {
                response.error = Some(serde_json::json!({
                    "code": -32601,
                    "message": format!("Method not found: {}", body.method)
                }));
            }
        }

        let mut headers = HeaderMap::new();
        headers.insert(
            header::CONTENT_TYPE,
            header::HeaderValue::from_static("text/event-stream"),
        );
        headers.insert(
            header::CACHE_CONTROL,
            header::HeaderValue::from_static("no-cache"),
        );
        headers.insert(
            header::CONNECTION,
            header::HeaderValue::from_static("keep-alive"),
        );

        let serialized_response = serde_json::to_string(&response).unwrap();
        let sse_payload = format!("event: message\ndata: {}\n\n", serialized_response);

        (StatusCode::OK, headers, sse_payload).into_response()
    } else {
        // Modern AG-UI flow
        info!("Handling via modern AG-UI flow");

        let thread_id = value
            .get("threadId")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        let run_id = value
            .get("runId")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();

        let mut history = Vec::new();
        if let Some(msgs_arr) = value.get("messages").and_then(|v| v.as_array()) {
            for msg_val in msgs_arr {
                let role = msg_val
                    .get("role")
                    .and_then(|v| v.as_str())
                    .unwrap_or("user")
                    .to_string();
                let content = msg_val
                    .get("content")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let id = msg_val
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();

                let parts = vec![Part {
                    kind: "text".to_string(),
                    text: content,
                    data: None,
                    metadata: None,
                }];

                history.push(Message {
                    role,
                    parts,
                    message_id: id,
                    context_id: Some(thread_id.clone()),
                    task_id: Some(run_id.clone()),
                });
            }
        }

        let (tx, rx) =
            tokio::sync::mpsc::channel::<Result<axum::body::Bytes, std::convert::Infallible>>(100);
        let http_client = state.http_client.clone();
        let thread_id_clone = thread_id.clone();
        let run_id_clone = run_id.clone();

        tokio::spawn(async move {
            // 1. Send RunStarted
            let run_started = serde_json::json!({
                "type": "RUN_STARTED",
                "threadId": thread_id_clone,
                "runId": run_id_clone
            });
            let _ = tx
                .send(Ok(axum::body::Bytes::from(format!(
                    "data: {}\n\n",
                    run_started
                ))))
                .await;

            // 2. Call Gemini
            let reply_text = execute_gemini_query(&http_client, &history).await;

            // 3. Send TextMessageStart
            let assistant_message_id = Uuid::new_v4().to_string();
            let text_message_start = serde_json::json!({
                "type": "TEXT_MESSAGE_START",
                "messageId": assistant_message_id,
                "role": "assistant"
            });
            let _ = tx
                .send(Ok(axum::body::Bytes::from(format!(
                    "data: {}\n\n",
                    text_message_start
                ))))
                .await;

            // 4. Stream reply_text in chunks
            let chars: Vec<char> = reply_text.chars().collect();
            let chunk_size = 30;
            let mut idx = 0;
            while idx < chars.len() {
                let end = std::cmp::min(idx + chunk_size, chars.len());
                let chunk: String = chars[idx..end].iter().collect();
                idx = end;

                let text_message_content = serde_json::json!({
                    "type": "TEXT_MESSAGE_CONTENT",
                    "messageId": assistant_message_id,
                    "delta": chunk
                });
                let _ = tx
                    .send(Ok(axum::body::Bytes::from(format!(
                        "data: {}\n\n",
                        text_message_content
                    ))))
                    .await;

                tokio::time::sleep(tokio::time::Duration::from_millis(15)).await;
            }

            // 5. Send TextMessageEnd
            let text_message_end = serde_json::json!({
                "type": "TEXT_MESSAGE_END",
                "messageId": assistant_message_id
            });
            let _ = tx
                .send(Ok(axum::body::Bytes::from(format!(
                    "data: {}\n\n",
                    text_message_end
                ))))
                .await;

            // 6. Send RunFinished
            let run_finished = serde_json::json!({
                "type": "RUN_FINISHED",
                "threadId": thread_id_clone,
                "runId": run_id_clone
            });
            let _ = tx
                .send(Ok(axum::body::Bytes::from(format!(
                    "data: {}\n\n",
                    run_finished
                ))))
                .await;
        });

        let body_stream = tokio_stream::wrappers::ReceiverStream::new(rx);
        let mut headers = HeaderMap::new();
        headers.insert(
            header::CONTENT_TYPE,
            header::HeaderValue::from_static("text/event-stream"),
        );
        headers.insert(
            header::CACHE_CONTROL,
            header::HeaderValue::from_static("no-cache"),
        );
        headers.insert(
            header::CONNECTION,
            header::HeaderValue::from_static("keep-alive"),
        );

        (
            StatusCode::OK,
            headers,
            axum::body::Body::from_stream(body_stream),
        )
            .into_response()
    }
}

// Direct interface with Gemini API and tool routing
async fn execute_gemini_query(client: &reqwest::Client, history: &[Message]) -> String {
    let api_key = std::env::var("GEMINI_API_KEY")
        .or_else(|_| std::env::var("GOOGLE_API_KEY"))
        .unwrap_or_default();

    if api_key.is_empty() {
        return "Error: Gemini API Key is missing in environment.".to_string();
    }

    let model = std::env::var("GENAI_MODEL").unwrap_or_else(|_| "gemini-2.5-flash".to_string());
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
        model, api_key
    );

    // Build model content structures from message history
    let mut contents = Vec::new();
    for msg in history {
        let role = match msg.role.as_str() {
            "user" => "user",
            "agent" | "model" => "model",
            _ => "user",
        };

        let mut parts_vals = Vec::new();
        for part in &msg.parts {
            parts_vals.push(serde_json::json!({
                "text": part.text
            }));
        }

        contents.push(serde_json::json!({
            "role": role,
            "parts": parts_vals
        }));
    }

    let today_str = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let dynamic_system_instruction = SYSTEM_INSTRUCTION.replace("{TODAYS_DATE}", &today_str);

    let mut turn_count = 0;
    const MAX_TURNS: usize = 15;

    while turn_count < MAX_TURNS {
        turn_count += 1;
        info!(
            "Executing Gemini query loop turn {}/{}...",
            turn_count, MAX_TURNS
        );

        // Include system instruction and declaration for the get_exchange_rate tool
        let payload = serde_json::json!({
            "contents": contents,
            "systemInstruction": {
                "parts": [{ "text": dynamic_system_instruction }]
            },
            "tools": [{
                "functionDeclarations": [
                    {
                        "name": "get_exchange_rate",
                        "description": "Use this to get the exchange rate for a specific date or 'latest'. To get historical trend data (e.g. for building a LineChart), call this tool multiple times sequentially or in parallel with different historical dates.",
                        "parameters": {
                            "type": "OBJECT",
                            "properties": {
                                "currency_from": {
                                    "type": "STRING",
                                    "description": "The currency to convert from (e.g., \"USD\")."
                                },
                                "currency_to": {
                                    "type": "STRING",
                                    "description": "The currency to convert to (e.g., \"EUR\")."
                                },
                                "currency_date": {
                                    "type": "STRING",
                                    "description": "The date for the exchange rate or \"latest\". Defaults to \"latest\"."
                                }
                            },
                            "required": ["currency_from", "currency_to"]
                        }
                    }
                ]
            }]
        });

        info!(
            "Sending request to Gemini generateContent (turn {})...",
            turn_count
        );
        let res = client.post(&url).json(&payload).send().await;

        match res {
            Ok(resp) => {
                if resp.status().is_success() {
                    if let Ok(data) = resp.json::<Value>().await {
                        info!("Gemini response (turn {}): {}", turn_count, data);
                        if let Some(candidates) = data.get("candidates").and_then(|c| c.as_array())
                        {
                            if let Some(first) = candidates.first() {
                                if let Some(content) = first.get("content") {
                                    if let Some(parts) =
                                        content.get("parts").and_then(|p| p.as_array())
                                    {
                                        let mut has_function_call = false;
                                        let mut function_responses = Vec::new();

                                        for part in parts {
                                            if let Some(func_call) = part.get("functionCall") {
                                                has_function_call = true;
                                                let func_name = func_call
                                                    .get("name")
                                                    .and_then(|n| n.as_str())
                                                    .unwrap_or_default();
                                                if func_name == "get_exchange_rate" {
                                                    let args = func_call
                                                        .get("args")
                                                        .cloned()
                                                        .unwrap_or(serde_json::json!({}));
                                                    info!(
                                                        "Gemini requested function call (turn {}): get_exchange_rate with args {:?}",
                                                        turn_count, args
                                                    );

                                                    // Invoke the local MCP toolset
                                                    let mcp_result = call_mcp_tool(
                                                        client,
                                                        "get_exchange_rate",
                                                        args.clone(),
                                                    )
                                                    .await;

                                                    // Construct functionResponse part
                                                    function_responses.push(serde_json::json!({
                                                        "functionResponse": {
                                                            "name": "get_exchange_rate",
                                                            "response": {
                                                                "name": "get_exchange_rate",
                                                                "content": mcp_result
                                                            }
                                                        }
                                                    }));
                                                }
                                            }
                                        }

                                        if has_function_call {
                                            if function_responses.is_empty() {
                                                return "Error: Unsupported tool call requested."
                                                    .to_string();
                                            }

                                            // Push model turn and function responses turn to contents
                                            contents.push(content.clone());
                                            contents.push(serde_json::json!({
                                                "role": "user",
                                                "parts": function_responses
                                            }));

                                            // Go back to the loop to let the model process the function responses
                                            continue;
                                        }

                                        // No function calls, look for text
                                        let mut final_text = String::new();
                                        for part in parts {
                                            if let Some(text) =
                                                part.get("text").and_then(|t| t.as_str())
                                            {
                                                final_text.push_str(text);
                                            }
                                        }

                                        if !final_text.is_empty() {
                                            return final_text;
                                        }
                                    }
                                }
                            }
                        }
                    }
                } else {
                    let status = resp.status();
                    let err_body = resp.text().await.unwrap_or_default();
                    error!(
                        "Gemini API responded with error (turn {}): {:?}",
                        turn_count, err_body
                    );
                    return format!(
                        "Error: Gemini API responded with error status {}: {}",
                        status, err_body
                    );
                }
            }
            Err(err) => {
                error!(
                    "HTTP request to Gemini API failed (turn {}): {:?}",
                    turn_count, err
                );
                return format!("Error: HTTP request to Gemini API failed: {:?}", err);
            }
        }
    }

    "Error: Exceeded maximum tool execution turns.".to_string()
}

// Call local MCP Server
async fn call_mcp_tool(client: &reqwest::Client, name: &str, arguments: Value) -> Value {
    let mcp_url =
        std::env::var("MCP_SERVER_URL").unwrap_or_else(|_| "http://127.0.0.1:8080/mcp".to_string());

    let payload = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": name,
            "arguments": arguments
        }
    });

    info!("Calling MCP server tools/call at {}...", mcp_url);
    let res = client.post(&mcp_url).json(&payload).send().await;

    match res {
        Ok(resp) => {
            let status = resp.status();
            info!("MCP server responded with status: {}", status);
            if status.is_success() {
                match resp.text().await {
                    Ok(body_text) => {
                        info!("MCP server raw response body: {}", body_text);
                        let json_str = if body_text.contains("data: ") {
                            body_text
                                .lines()
                                .find(|line| line.starts_with("data: "))
                                .map(|line| line["data: ".len()..].trim())
                                .unwrap_or(&body_text)
                        } else {
                            &body_text
                        };

                        match serde_json::from_str::<Value>(json_str) {
                            Ok(mcp_response) => {
                                info!("MCP server parsed JSON response: {:?}", mcp_response);
                                if let Some(result) = mcp_response.get("result") {
                                    return result.clone();
                                } else {
                                    error!(
                                        "MCP response did not contain 'result' field: {:?}",
                                        mcp_response
                                    );
                                }
                            }
                            Err(e) => {
                                error!(
                                    "Failed to parse MCP response JSON: {:?}. Raw string: {}",
                                    e, json_str
                                );
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to read MCP response body text: {:?}", e);
                    }
                }
            } else {
                error!(
                    "MCP server returned error status: {}, body: {:?}",
                    status,
                    resp.text().await
                );
            }
        }
        Err(err) => {
            error!("MCP tool invocation HTTP request failed: {:?}", err);
        }
    }

    serde_json::json!({ "error": "MCP server call failed" })
}
