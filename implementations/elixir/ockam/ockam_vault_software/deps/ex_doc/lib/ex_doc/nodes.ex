defmodule ExDoc.ModuleNode do
  @moduledoc """
  Structure that represents a module.
  """

  defstruct id: nil,
            title: nil,
            nested_context: nil,
            nested_title: nil,
            module: nil,
            group: nil,
            deprecated: nil,
            doc: nil,
            rendered_doc: nil,
            doc_line: nil,
            function_groups: [],
            docs: [],
            typespecs: [],
            source_path: nil,
            source_url: nil,
            type: nil,
            language: nil

  @type t :: %__MODULE__{
          id: nil | String.t(),
          title: nil | String.t(),
          nested_context: nil | String.t(),
          nested_title: nil | String.t(),
          module: nil | String.t(),
          group: nil | String.t(),
          deprecated: nil | String.t(),
          function_groups: list(String.t()),
          docs: list(),
          doc: term(),
          rendered_doc: nil | String.t(),
          doc_line: non_neg_integer(),
          typespecs: list(),
          source_path: nil | String.t(),
          source_url: nil | String.t(),
          type: nil | atom(),
          language: module()
        }
end

defmodule ExDoc.FunctionNode do
  @moduledoc """
  Structure that represents an individual function.
  """

  defstruct id: nil,
            name: nil,
            arity: 0,
            defaults: [],
            deprecated: nil,
            doc: nil,
            rendered_doc: nil,
            type: nil,
            signature: nil,
            specs: [],
            annotations: [],
            group: nil,
            doc_line: nil,
            source_path: nil,
            source_url: nil

  @type t :: %__MODULE__{
          id: nil | String.t(),
          name: nil | String.t(),
          arity: non_neg_integer,
          defaults: non_neg_integer,
          doc: term(),
          rendered_doc: nil | String.t(),
          doc_line: non_neg_integer,
          source_path: nil | String.t(),
          source_url: nil | String.t(),
          group: nil | String.t(),
          type: nil | String.t(),
          signature: nil | String.t(),
          specs: list(),
          annotations: list(),
          deprecated: nil | String.t()
        }
end

defmodule ExDoc.TypeNode do
  @moduledoc """
  Structure that represents an individual type.
  """

  defstruct id: nil,
            name: nil,
            arity: 0,
            type: nil,
            deprecated: nil,
            doc: nil,
            rendered_doc: nil,
            doc_line: nil,
            source_path: nil,
            source_url: nil,
            spec: nil,
            signature: nil,
            annotations: []

  @type t :: %__MODULE__{
          id: nil | String.t(),
          name: nil | String.t(),
          arity: non_neg_integer,
          type: nil | String.t(),
          spec: nil | String.t(),
          deprecated: nil | String.t(),
          doc: term(),
          rendered_doc: nil | String.t(),
          doc_line: non_neg_integer,
          signature: nil | String.t(),
          source_url: nil | String.t(),
          source_path: nil | String.t(),
          annotations: list()
        }
end
