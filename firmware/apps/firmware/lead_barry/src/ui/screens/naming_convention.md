# Naming Conventions

This module follows specific naming conventions:

| Prefix       | Description                  | Example                |
| ------------ | ---------------------------- | ---------------------- |
| `Sv`         | Screen View implementation.  | \[`SvMessage`\].       |
| `Dm`         | Data Model type              | \[`DmMessage`\]        |
| `DataModel`  | Data Model trait             | \[`DataModelMessage`\] |
| `DataModelT` | Data Model generic parameter | \[`DataModelT`\]       |

Screen view types follow the pattern: `Sv` + ScreenViewName

Data Model types follow the pattern: `Dm` + ScreenViewName. For example, `DmMessage`
is a data model type for a message screen view.

The types that subordinate Data Model types follow the pattern: `Dm` + ScreenViewName +
SubordinateTypeName. For example, `DmWifiApData` is a data model type for a WiFi AP screen
view, and `DmWifiApDataClientInfo` is a subordinate type that represents client information
for the WiFi AP data model.

The screen view types can have data model types as generic parameters that implement
the necessary traits to provide data to the screen. For example, `SvMessageImpl<DataModelT>`
is a screen view that takes a generic parameter `DataModelT` which must implement the
`DataModelMessage` trait to provide the title and message data needed for the screen.

The Screen View can be generic over the data model, but the data model must implement the
appropriate trait to be used with that screen view. For example, `SvMessageImpl<DataModelT>`
can be used with any `DataModelT` that implements the `DataModelMessage` trait, allowing
for flexibility in the data models that can be used with the message screen view.

The name generic Screen View types follow the pattern: `Sv` + ScreenViewName + `Impl`. For
example, `SvMessageImpl<DataModelT>` is a generic screen view implementation for a message
screen that can work with any data model that implements the `DataModelMessage` trait.

Data model traits follow the pattern: `DataModel` + ScreenViewName. For example,
`DataModelMessage` is a trait that defines the data model for a message screen view.
