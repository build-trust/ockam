import SwiftUI

struct Autocomplete: View {
    @FocusState private var isFocused: Bool

    @State var suggestions: Array<String>
    @State var maxSuggestions: Int = 3
    @State var label: String = ""
    @Binding var value: String

    @State private var showingSuggestions: Bool = false
    @State private var selectedValue: String = ""
    @State private var selected: Int = -1

    var body: some View {
        //keep only the first 5
        let filteredSuggestions = suggestions.filter {
            value == "" || $0.contains(value)
        }.prefix(3)

        VStack(spacing: 0) {
            HStack(spacing: 0) {
                // In order to keep the overlay aligned with the TextField
                // we manually handle the label as it was embedded in a Form
                Text(label).padding(0)
                TextField("", text: $value)
                    .focused($isFocused)
                    .onChange(of: value) { newValue in
                        if value != selectedValue {
                            showingSuggestions = true
                        }
                    }
                    .onSubmit({
                        if selected >= 0 {
                            selectedValue = filteredSuggestions[selected]
                            value = selectedValue
                            selected = -1
                        } else {
                            selectedValue = value
                        }
                        showingSuggestions = false
                    })
                    .onAppear {
                        NSEvent.addLocalMonitorForEvents(matching: .keyDown) {
                            if !isFocused {
                                return $0
                            }

                            if $0.keyCode == 125 { // handling down arrow key press
                                if selected < filteredSuggestions.count - 1 {
                                    selected += 1
                                    showingSuggestions = true
                                }
                                return nil
                            } else if $0.keyCode == 126 { // handling up arrow key press
                                if selected >= 0 {
                                    selected -= 1
                                }
                                if selected < 0 {
                                    showingSuggestions = false
                                }
                                return nil
                            } else {
                                return $0
                            }
                        }
                    }
                    .padding(.bottom, 0)
                    .overlay(alignment: .top, content: {
                        if showingSuggestions {
                            HStack {
                                VStack(spacing: 0) {
                                    ForEach(filteredSuggestions, id: \.self) { suggestion in
                                        let index = filteredSuggestions.firstIndex(of: suggestion)

                                        if selected == index {
                                            ClickableMenuEntry(
                                                selected: true,
                                                text: suggestion,
                                                action: {
                                                    showingSuggestions = false
                                                    value = suggestion
                                                    selectedValue = suggestion
                                                }
                                            )
                                        } else {
                                            ClickableMenuEntry(
                                                selected: false,
                                                text: suggestion,
                                                action: {
                                                    showingSuggestions = false
                                                    value = suggestion
                                                    selectedValue = suggestion
                                                }
                                            )
                                        }
                                    }
                                }
                            }
                            .background(.background)
                            .offset(x: 6, y: 25)
                        }
                    })
            }
        }
        .zIndex(10)
    }
}

struct PreviewWrapper: View {
    @State var value1 = ""
    @State var value2 = ""

    var body: some View {
        // When using the Form the label is used next the field
        // instead of as placeholder
        Form {
            TextField("Name", text: $value1)
            Autocomplete(
                suggestions: ["one","two","three","four"],
                label: "Label",
                value: $value2
            )
            Text("...zIndex check...")
        }
        .frame(width: 300, height: 250)
    }
}

struct Autocomplete_Previews: PreviewProvider {
    static var previews: some View {
        PreviewWrapper()
    }
}
