![Bunt](https://raw.github.com/rrrene/bunt/master/assets/logo.png)

Enables 256 color ANSI coloring in the terminal and gives you the ability to alias colors to more semantic and application-specfic names.


## Installation

It's available via Hex:

  1. Add bunt to your list of dependencies in `mix.exs`:

        def deps do
          [{:bunt, "~> 0.1.0"}]
        end

  2. Ensure bunt is started before your application:

        def application do
          [applications: [:bunt]]
        end



## Usage



### 256 colors

![Colors](https://raw.github.com/rrrene/bunt/master/assets/colors.png)

`IO.ANSI` provides an interface to write text to the terminal in eight different colors like this:

    ["Hello, ", :red, :bright, "world!"]
    |> IO.ANSI.format
    |> IO.puts

This will put the word "world!" in bright red.

To cause as little friction as possible, the interface of `Bunt.ANSI` is 100% adapted from `IO.ANSI`.

We can use `Bunt` in the same way:

    ["Hello, ", :color202, :bright, "world!"]
    |> Bunt.ANSI.format
    |> IO.puts

which puts a bright orange-red `"world!"` on the screen.

`Bunt` also provides a shortcut so we can skip the `format` call.

    ["Hello, ", :color202, :bright, "world!"]
    |> Bunt.puts

and since nobody can remember that `:color202` is basically `:orangered`, you can use `:orangered` directly.



### Named colors

The following colors were given names, so you can use them in style:

    [:gold, "Look, it's really gold text!"]
    |> Bunt.puts

Replace `:gold` with any of these values:

    darkblue      mediumblue    darkgreen     darkslategray darkcyan
    deepskyblue   springgreen   aqua          dimgray       steelblue
    darkred       darkmagenta   olive         chartreuse    aquamarine
    greenyellow   chocolate     goldenrod     lightgray     beige
    lightcyan     fuchsia       orangered     hotpink       darkorange
    coral         orange        gold          khaki         moccasin
    mistyrose     lightyellow

You can see all supported colors by cloning the repo and running:

    $ mix run script/colors.exs

### User-defined color aliases

But since all these colors are hard to remember, you can alias them in your config.exs:

    # I tend to start the names of my color aliases with an underscore
    # but this is, naturally, not a must.

    config :bunt, color_aliases: [_cupcake: :color205]

Then you can use these keys instead of the standard colors in your code:

    [:_cupcake, "Hello World!"]
    |> Bunt.puts

Use this to give your colors semantics. They get easier to change later that way. (A colleague of mine shouted "It's CSS for console applications!" when he saw this and although that is ... well, not true, I really like the sentiment! :+1:)




## Contributing

1. [Fork it!](http://github.com/rrrene/bunt/fork)
2. Create your feature branch (`git checkout -b my-new-feature`)
3. Commit your changes (`git commit -am 'Add some feature'`)
4. Push to the branch (`git push origin my-new-feature`)
5. Create new Pull Request



## Author

René Föhring (@rrrene)



## License

Bunt is released under the MIT License. See the LICENSE file for further
details.

"Elixir" and the Elixir logo are copyright (c) 2012 Plataformatec.

Elixir source code is released under Apache 2 License.

Check NOTICE, ELIXIR-LICENSE and LICENSE files for more information.
