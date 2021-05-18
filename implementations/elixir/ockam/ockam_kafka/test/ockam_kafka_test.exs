defmodule OckamKafkaTest do
  use ExUnit.Case
  doctest OckamKafka

  test "greets the world" do
    assert OckamKafka.hello() == :world
  end
end
