# Panduan WinWeaver

WinWeaver menyiapkan instalasi Windows 11 Pro tanpa pengawasan untuk grup kerja kecil dan menjaga
konfigurasi PC yang sudah terinstal. Profil menentukan aturan katalog mana yang aktif, dengan parameter
apa, dan apa isi formulir instalasi; program membuat `autounattend.xml` dari profil tersebut.

Halaman ini terbuka saat program pertama kali dimulai, dan selanjutnya dari menu atau dengan F1. Halaman
ini adalah file Markdown; halaman dokumentasi terbuka dengan cara yang sama, masing-masing di tabnya sendiri.

## Langkah pertama

1. Buka preset: **Menu, Baru, Profil dari preset Office**. Office berisi default katalog;
   Strict menambahkan pembatasan, Laptop cocok untuk PC portabel, Home adalah daftar izin untuk PC rumah.
2. Telusuri pohon di sebelah kiri. Kotak centang mengaktifkan atau menonaktifkan aturan; kotak centang grup
   mengaktifkan atau menonaktifkan semua aturannya sekaligus; aturan yang berwarna abu-abu menunggu aturan
   yang diperlukannya. Panel kanan menampilkan parameter aturan yang dipilih dan deskripsinya.
3. Isi formulir **Instalasi**, **Akun**, dan **Bahasa** di bawah akar pertama.
4. Periksa profil dengan F7. Pesan muncul di bawah baris navigasi; klik pada pesan akan memilih aturan
   atau formulir yang dimaksud.
5. Buat file jawaban dengan F9: `autounattend.xml` disimpan di folder `output` di samping file
   program, bersama skrip yang disematkannya agar dapat dilihat. Salin file tersebut ke akar flash disk USB
   instalasi.
6. Simpan profil dengan Ctrl+S. Profil adalah file `.wwprofile` di folder `profiles` di samping file
   program, atau di mana pun yang Anda pilih dengan Simpan sebagai.

## Jendela

- **Tab**: setiap dokumen yang terbuka adalah tab: profil, perbandingan, panduan ini, penanda, server MCP,
  pengaturan. Tombol menu di ujung kiri baris tab berisi Baru, Buka, Simpan, Bandingkan, panduan,
  server MCP, penanda, Pengaturan, dan Keluar.
- **Bilah perintah**: Baru, Buka, Simpan, Periksa, Buat, PC Ini, Bandingkan. Di jendela yang sempit,
  bilah ini hanya menampilkan ikon, di baris navigasi.
- **Baris navigasi**: Kembali dan Maju berpindah di antara item yang telah Anda pilih; jalur navigasi
  menunjukkan posisi Anda dan dapat melompat ke tingkat mana pun; Penanda menandai posisi; Cari memfilter
  pohon saat Anda mengetik dan mengingat apa yang pernah Anda cari.
- **Pohon dan panel detail**: pohon di sebelah kiri, item yang dipilih di sebelah kanan. Deskripsi
  diciutkan dengan Ctrl+D. Aturan dari grup yang dipilih menampilkan tombol alih di ujung barisnya.
- **Bilah status**: status dokumen, versi katalog, server MCP, dan pesan terakhir.

## PC Ini

**PC Ini** di bilah perintah membaca pengaturan komputer ini melalui Windows, tanpa mengubah
apa pun:

- **Baca PC ini ke profil baru**: setiap aturan yang dapat dinilai pada Windows yang sedang berjalan menjadi
  aktif (berlaku), nonaktif (berlaku sebagian atau tidak berlaku), atau mempertahankan statusnya (tidak dapat
  dibaca, atau sama seperti pada Windows yang bersih); formulir mengambil edisi, zona waktu, dan bahasa.
- **Baca pilihan ke profil ini** dan **Periksa pilihan di PC ini** bekerja pada aturan atau grup
  yang dipilih.
- **Simpan skrip penerapan**, **skrip pengembalian ke default**, atau **skrip audit** menulis skrip
  PowerShell untuk PC ini atau PC lain. Jalankan skrip penerapan atau pengembalian sebagai administrator,
  terlebih dahulu di PC uji; skrip ini menyimpan cadangan yang dapat dipulihkan oleh `Undo-Apply.ps1` di
  sampingnya. Laporan skrip audit dibuka dengan **Buka laporan audit** dan menjadi profil.

Aturan yang hanya bekerja selama instalasi atau saat pengguna masuk pertama kali tidak dibaca.

## Membandingkan profil

**Menu, Bandingkan dengan file** atau **Bandingkan dengan preset** membuka tab perbandingan: setiap perbedaan
antara profil aktif dan profil lainnya, dalam tiga kolom. "Terapkan sisi kanan" mengatur nilai-nilai tersebut
di profil tempat perbandingan dibuat; "Simpan sebagai file perbedaan" menyimpan perbedaan sebagai file
`.wwdiff` yang nanti dapat diterapkan ke profil mana pun.

## Membuka file lain

Buka menerima profil (`.wwprofile`), perbandingan (`.wwdiff`), file penanda (`.wwmarks`), halaman Markdown,
file jawaban yang dibuat oleh WinWeaver atau WinKickOff (profil yang tersemat di dalamnya), profil WinKickOff,
dan laporan audit (`.json`).

## Server MCP

**Menu, Server MCP** membuka tab server yang digunakan agen AI untuk membaca profil dan, bila mode
mengizinkan, mengubahnya. Server dimulai dalam mode baca (read); mode edit memungkinkannya mengubah profil
di memori, mode file (files) memungkinkannya menyimpan profil dan menulis file jawaban. Membaca PC ini
memerlukan izin tersendiri, yang nonaktif setiap kali program dimulai.

## Keyboard

| Tombol | Tindakan |
|---|---|
| Ctrl+N, Ctrl+O, Ctrl+S, Ctrl+Shift+S | Baru, Buka, Simpan, Simpan sebagai |
| F7, F9 | Periksa profil, buat file jawaban |
| Ctrl+F, Esc | Cari, kosongkan dan tutup pencarian |
| Alt+Left, Alt+Right | Kembali, Maju |
| Ctrl+B | Tandai posisi |
| Ctrl+D | Ciutkan atau tampilkan deskripsi |
| Ctrl+W, Ctrl+Tab, Ctrl+Shift+Tab | Tutup tab, tab berikutnya, tab sebelumnya |
| Esc | Batalkan konfirmasi perubahan PC ini |
| Ctrl+, | Pengaturan |
| F1 | Panduan ini |

## File di samping file program

`settings.json` (bahasa, tema, file terbaru, tab dari sesi terakhir), `profiles/` (profil
Anda), `output/` (file jawaban dan skrip), `drafts/` (perubahan yang belum disimpan, dipulihkan saat
program dimulai berikutnya), `logs/` (apa yang dilakukan program), `bookmarks.wwmarks`. Tidak ada yang
ditulis di tempat lain.

## Belum tersedia

Penerapan pengaturan ke PC ini langsung dari program (sementara itu gunakan skrip penerapan) dan impor
templat kebijakan (ADMX) akan hadir di versi berikutnya.
