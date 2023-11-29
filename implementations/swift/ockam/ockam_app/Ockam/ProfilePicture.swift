import SwiftUI

struct ProfilePicture: View {
    @State var url: String?
    @State var placeholder = "person"
    @State var size: CGFloat = 48

    var body: some View {
        if let url = url {
            AsyncImage(
                url: URL(string: url),
                content: { image in
                    image.resizable()
                        .aspectRatio(contentMode: .fit)
                        .clipShape(RoundedRectangle(cornerSize: CGSize(width: 10, height: 10)))
                },
                placeholder: {
                    Image(systemName: placeholder)
                        .resizable()
                        .aspectRatio(contentMode: .fit)
                        .frame(maxWidth: size, maxHeight: size)
                }
            ).frame(width: size, height: size)
        } else {
            Image(systemName: placeholder)
                .resizable()
                .aspectRatio(contentMode: .fit)
                .frame(width: size, height: size)
        }
    }
}
