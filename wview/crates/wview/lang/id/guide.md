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
  **Berikutnya**, gambar di folder yang sama; untuk animasi **Putar** (atau **Jeda**), **Bingkai
  sebelumnya** dan **Bingkai berikutnya**; untuk file dengan beberapa halaman (TIFF, ICO)
  **Halaman sebelumnya** dan **Halaman berikutnya**.
- Klik pada gambar memberinya tombol: panah menggeser gambar yang lebih besar dari jendela, jika
  tidak panah kiri dan kanan mengambil gambar sebelumnya atau berikutnya di folder, seperti yang
  selalu dilakukan Page Up dan Page Down; Home dan End yang pertama dan terakhir; + dan - mengubah
  zoom bertahap, 0 menyesuaikan gambar, 1 sampai 4 menampilkan 100% sampai 400%; Spasi memutar
  atau menjeda animasi, koma dan titik pindah ke bingkai atau halaman sebelum atau sesudahnya. Tab
  atau Escape mengembalikan tombol ke jendela.
- Foto diputar sesuai cara kameranya dipegang (orientasi EXIF). Gambar besar didekode saat tabnya
  sudah terbuka dan digambar per bagian dengan detail yang dibutuhkan zoom; gambar di atas batas
  pengaturan didekode lebih kecil, dan bilah status menyebutkannya.

## File biner

Program dan pustaka Windows (`.exe`, `.dll`, `.sys` dan file Portable Executable lainnya) terbuka
sebagai pohon strukturnya: header, bagian, impor dan ekspor, setiap bidang dengan nilai, offset
dan ukurannya.

- Di kiri pohon: klik atau panah memilih simpul, kanan dan kiri membuka dan menutupnya. Di kanan
  byte file, 16 per baris dengan ASCII-nya: byte simpul yang dipilih ditandai, dan klik pada
  sebuah byte memilih simpul terkecil yang memuatnya.
- **Teks** di bilah menampilkan pohon sebagai halaman, dengan pencarian tab (Ctrl+F); **Pohon dan
  byte** kembali.
- File tetap di disk: tampilan membaca blok yang ditampilkannya. Simpul yang tidak dimuat utuh
  oleh file (file pendek atau rusak) memiliki tanda peringatan dan menyebutkan alasannya.
- Selain header: sumber daya (jenis, nama dan bahasanya, satu tingkat dibaca saat dibuka),
  pengecualian x64, sertifikat, relokasi, direktori debug dengan jalur PDB, TLS, konfigurasi muat,
  impor tertunda dan header .NET dengan aliran metadatanya.
- **Simpan struktur** menulis pohon sebagai file `.wwbin`, secara bawaan di samping file: setiap
  bidang dengan nilainya, setiap rentang byte mentah dengan SHA-256-nya, file dengan ukuran dan
  SHA-256-nya, tanpa satu pun bytenya.
- Format lain datang sebagai deskripsi, file `.wwformat` di folder `plugins\formats`; Pengaturan,
  Plugin menampilkan format yang terbaca dan file yang tidak terbaca.

## Pengaturan

**Pengaturan**, **Plugin** menampilkan plugin dengan pengaturannya: file teks terbesar yang dibuka;
piksel yang dihaluskan di atas 100%, orientasi EXIF, animasi yang diputar saat dibuka dan gambar
terbesar yang didekode utuh (64, 256 atau 1024 megapiksel); byte per baris, blok file biner dan
cache bloknya. Pengaturan disimpan di `wview.settings.json` di samping program.

## Baris perintah

- `wview FILE...` membuka file di tab.
- `wview --plugins` menampilkan plugin dan formatnya.
- `wview --probe FILE` membuka file dan menggambarnya tanpa jendela, dengan waktunya.
- `wview --help` menampilkan opsi.
