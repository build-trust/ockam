defmodule Dialyxir.Warnings do
  @warnings Enum.into(
              [
                Dialyxir.Warnings.AppCall,
                Dialyxir.Warnings.Apply,
                Dialyxir.Warnings.BinaryConstruction,
                Dialyxir.Warnings.Call,
                Dialyxir.Warnings.CallToMissingFunction,
                Dialyxir.Warnings.CallWithOpaque,
                Dialyxir.Warnings.CallWithoutOpaque,
                Dialyxir.Warnings.CallbackArgumentTypeMismatch,
                Dialyxir.Warnings.CallbackInfoMissing,
                Dialyxir.Warnings.CallbackMissing,
                Dialyxir.Warnings.CallbackSpecArgumentTypeMismatch,
                Dialyxir.Warnings.CallbackSpecTypeMismatch,
                Dialyxir.Warnings.CallbackTypeMismatch,
                Dialyxir.Warnings.ContractDiff,
                Dialyxir.Warnings.ContractRange,
                Dialyxir.Warnings.ContractSubtype,
                Dialyxir.Warnings.ContractSupertype,
                Dialyxir.Warnings.ContractWithOpaque,
                Dialyxir.Warnings.ExactEquality,
                Dialyxir.Warnings.ExtraRange,
                Dialyxir.Warnings.FunctionApplicationArguments,
                Dialyxir.Warnings.FunctionApplicationNoFunction,
                Dialyxir.Warnings.GuardFail,
                Dialyxir.Warnings.GuardFailPattern,
                Dialyxir.Warnings.ImproperListConstruction,
                Dialyxir.Warnings.InvalidContract,
                Dialyxir.Warnings.MapUpdate,
                Dialyxir.Warnings.MissingRange,
                Dialyxir.Warnings.NegativeGuardFail,
                Dialyxir.Warnings.NoReturn,
                Dialyxir.Warnings.OpaqueGuard,
                Dialyxir.Warnings.OpaqueEquality,
                Dialyxir.Warnings.OpaqueMatch,
                Dialyxir.Warnings.OpaqueNonequality,
                Dialyxir.Warnings.OpaqueTypeTest,
                Dialyxir.Warnings.OverlappingContract,
                Dialyxir.Warnings.PatternMatch,
                Dialyxir.Warnings.PatternMatchCovered,
                Dialyxir.Warnings.RaceCondition,
                Dialyxir.Warnings.RecordConstruction,
                Dialyxir.Warnings.RecordMatching,
                Dialyxir.Warnings.UnknownBehaviour,
                Dialyxir.Warnings.UnknownFunction,
                Dialyxir.Warnings.UnknownType,
                Dialyxir.Warnings.UnmatchedReturn,
                Dialyxir.Warnings.UnusedFunction
              ],
              %{},
              fn warning -> {warning.warning(), warning} end
            )

  @doc """
  Returns a mapping of the warning to the warning module.
  """
  def warnings(), do: @warnings
end
