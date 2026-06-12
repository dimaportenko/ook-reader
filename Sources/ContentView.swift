import SwiftUI

// A `View` is a value that describes a piece of UI. SwiftUI rebuilds it from
// state whenever that state changes. Keep them small and composable.
struct ContentView: View {
    var body: some View {
        VStack(spacing: 12) {
            Image(systemName: "book")
                .font(.system(size: 48))
                .foregroundStyle(.tint)
            Text("Ook Reader")
                .font(.largeTitle.bold())
            Text("Phase 2 scaffold — it builds, runs, and breakpoints work.")
                .font(.subheadline)
                .foregroundStyle(.secondary)
                .multilineTextAlignment(.center)
        }
        .padding()
    }
}

#Preview {
    ContentView()
}
