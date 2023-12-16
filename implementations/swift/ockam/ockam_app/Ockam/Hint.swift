import SwiftUI


struct Hint: View {
    @State var title: String = "Guide"
    @State var text: String

    init(_ text: String) {
        self.text = text
    }

    init(_ title: String, _ text: String) {
        self.title = title
        self.text = text
    }

    var body: some View {
        VStack {
            VStack(alignment: .leading) {
                HStack(alignment: .center){
                    Image(systemName: "info.circle")
                    Text(title)
                    Spacer()
                }
                .padding(.vertical, VerticalSpacingUnit)
                .padding(.horizontal, HorizontalSpacingUnit)
                .background(Color.blue.opacity(0.10))

                Text(text)
                    .padding(.vertical, VerticalSpacingUnit*2)
                    .padding(.horizontal, HorizontalSpacingUnit*2)
                    .fixedSize(horizontal: false, vertical: true)
            }
            .background(Color.blue.opacity(0.2))
        }
        .contentShape(Rectangle())
        .cornerRadius(10)
        .padding(.vertical, VerticalSpacingUnit)
        .padding(.horizontal, HorizontalSpacingUnit)
    }

}


struct Hint_Previews: PreviewProvider {
    static var previews: some View {
        Hint("Title!", "Hello world!")
    }
}
