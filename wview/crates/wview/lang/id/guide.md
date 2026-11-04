# wview

wview menampilkan file. Setiap format adalah plugin: DLL di folder `plugins` di samping
`wview.exe`. Tanpa file, jendela menampilkan daftar plugin dan apa yang dibuka masing-masing.

## Membuka file

- **Buka** (Ctrl+O), atau file yang diletakkan di jendela, membuka file di tab; `wview FILE...`
  membuka file dari baris perintah.
- File dengan format yang tidak dibuka plugin mana pun tetap tertutup, dan bilah status
  memberitahukannya.
- Ctrl+W menutup tab, Ctrl+Tab pindah ke tab berikutnya.

## Teks dan Markdown

Halaman Markdown (`.md`) tampil sebagai halaman, file teks (`.txt`, `.log`, `.ini`, `.cfg`,
`.csv`) apa adanya. Ctrl+F mencari dalam teks. **Pengaturan**, **Umum**: *Lipat baris sesuai lebar
tampilan* dan *Lebar halaman* mengubah tampilan teks.

## Gambar

Semua format yang didekode Windows di komputer ini: BMP, PNG, JPEG, GIF, TIFF, ICO, serta HEIF,
AVIF, WebP atau RAW kamera bila ekstensinya terpasang.

- Roda memperbesar di bawah penunjuk, menyeret menggeser gambar, klik ganda berpindah antara
  seluruh gambar dan 100%.
- Bilah: **Sesuaikan**, **100%**, **Perbesar**, **Perkecil**, serta **Sebelumnya** dan
  **Berikutnya**, gambar di folder yang sama.
- Klik pada gambar memberinya tombol: panah kiri dan kanan atau Page Up dan Page Down mengambil
  gambar sebelumnya atau berikutnya di folder, Home dan End yang pertama dan terakhir, + dan -
  mengubah zoom, 0 menyesuaikan gambar, 1 menampilkan 100%. Tab atau Escape mengembalikan tombol
  ke jendela.

## Pengaturan

**Pengaturan**, **Plugin** menampilkan plugin dengan pengaturannya: file teks terbesar yang
dibuka; piksel yang dihaluskan di atas 100% dan sisi terpanjang gambar yang didekode. Pengaturan
disimpan di `wview.settings.json` di samping program.

## Baris perintah

- `wview FILE...` membuka file di tab.
- `wview --plugins` menampilkan plugin dan formatnya.
- `wview --probe FILE` membuka file dan menggambarnya tanpa jendela, dengan waktunya.
- `wview --help` menampilkan opsi.
