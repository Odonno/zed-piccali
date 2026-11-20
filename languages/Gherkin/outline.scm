; Outline: Feature > Rule/Background/Scenario > Examples, named by the text
; after each keyword (the `context` child of the *_line nodes).

(feature
  (feature_header
    (feature_line
      (context) @name))) @item

(rule
  (rule_header
    (rule_line
      (context) @name))) @item

(background
  (background_line
    (context) @name)) @item

(scenario_definition
  (scenario
    (scenario_line
      (context) @name))) @item

(scenario_definition
  (scenario
    (scenario_outline_line
      (context) @name))) @item

(examples_definition
  (examples
    (examples_line
      (context) @name))) @item
