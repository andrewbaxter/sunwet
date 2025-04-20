declare type Direction = "up" | "down" | "left" | "right";
declare type Orientation =
  | "up_left"
  | "up_right"
  | "down_left"
  | "down_right"
  | "left_up"
  | "left_down"
  | "right_up"
  | "right_down";
declare type TransAlign = "start" | "middle" | "end";

type QueryId = string;

declare type LineSizeMode = "ellipsize" | "wrap";

declare type FieldOrLiteral =
  | { type: "field"; value: string }
  | { type: "literal"; value: string };

declare type QueryOrField =
  | { type: "field"; value: string }
  | {
      type: "query";
      value: { query: string; params: { [s: string]: FieldOrLiteral } };
    };

declare type WidgetTextLine = {
  data: FieldOrLiteral;
  prefix?: string;
  suffix?: string;
  size?: string;
  size_mode?: LineSizeMode;
  size_max?: string;
  orientation: Orientation;
  trans_align?: TransAlign;
  link?: FieldOrLiteral;
};

declare type WidgetImage = {
  data: FieldOrLiteral;
  width?: string;
  height?: string;
  trans_align?: TransAlign;
};

declare type WidgetPlayButton = {
  /// The media type (ex `sunwet/1/video`, `sunwet/1/audio`)
  media_field: FieldOrLiteral;
  name_field?: string;
  album_field?: string;
  artist_field?: string;
  cover_field?: string;
  direction?: Direction;
  trans_align?: TransAlign;
};

declare type DataRowsLayout = "other" | "table";

declare type WidgetDataRows = {
  /// Where to get the data for the sublist.
  data: QueryOrField;
  /// A field of the returned data that can be used as a unique key for
  /// saving/restoring position in playback.
  key_field: string;
  /// How the data rows are displayed.
  row_widget:
    | { type: "other"; gap?: string; direction: Direction; widget: Widget }
    | { type: "table"; orientation: Orientation; elements: Widget[] };
  trans_align?: TransAlign;
  x_scroll?: boolean;
};

declare type WidgetLayout = {
  direction: Direction;
  trans_align?: TransAlign;
  x_scroll?: boolean;
  elements: Widget[];
  gap?: string;
};

declare type Widget =
  | {
      type: "layout";
      widget: WidgetLayout;
    }
  | {
      type: "data_rows";
      widget: WidgetDataRows;
    }
  | {
      type: "text";
      widget: WidgetTextLine;
    }
  | {
      type: "image";
      widget: WidgetImage;
    }
  | {
      type: "play";
      widget: WidgetPlayButton;
    };

declare type QueryDefParameter = "text" | "number" | "bool" | "datetime";
