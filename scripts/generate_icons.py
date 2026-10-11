import os
from PIL import Image, ImageDraw, ImageFont

os.makedirs("assets", exist_ok=True)

def create_app_icon():
    # Render at 1024x1024 for pristine antialiasing
    size = 1024
    img = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)

    # 1. Background squircle / rounded tile
    # Deep twilight sapphire / slate gradient background
    margin = 48
    r = 200
    
    # Outer subtle drop shadow
    shadow = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    s_draw = ImageDraw.Draw(shadow)
    s_draw.rounded_rectangle(
        [margin, margin + 24, size - margin, size - margin + 24],
        radius=r,
        fill=(0, 0, 0, 80)
    )
    img = Image.alpha_composite(shadow, img)
    draw = ImageDraw.Draw(img)

    # Base squircle
    draw.rounded_rectangle(
        [margin, margin, size - margin, size - margin],
        radius=r,
        fill=(28, 37, 54, 255), # Deep navy/slate
        outline=(52, 69, 99, 255),
        width=12
    )

    # Subtle inner gloss / border accent
    draw.rounded_rectangle(
        [margin + 8, margin + 8, size - margin - 8, size - margin - 8],
        radius=r - 6,
        outline=(70, 95, 135, 120),
        width=6
    )

    # 2. Book Cover Background (rich warm mahogany/sepia back)
    # Open book layout
    center_x = size // 2
    book_top = 220
    book_bottom = 760
    book_left = 160
    book_right = size - 160

    # Book cover edge (slightly larger than pages)
    cover_left = book_left - 16
    cover_right = book_right + 16
    cover_bottom = book_bottom + 20
    draw.polygon([
        (cover_left, book_top + 40),
        (center_x, book_top + 80),
        (cover_right, book_top + 40),
        (cover_right, cover_bottom),
        (center_x, cover_bottom + 40),
        (cover_left, cover_bottom)
    ], fill=(120, 60, 25, 255)) # Warm leather spine/cover

    # Left page (creamy warm paper)
    left_page = [
        (book_left, book_top + 30),
        (center_x - 12, book_top + 70),
        (center_x - 12, book_bottom + 30),
        (book_left, book_bottom - 10)
    ]
    # Draw page stack depth
    draw.polygon([
        (book_left, book_top + 38),
        (center_x - 12, book_top + 78),
        (center_x - 12, book_bottom + 38),
        (book_left, book_bottom - 2)
    ], fill=(215, 205, 185, 255))

    # Main Left Page
    draw.polygon(left_page, fill=(248, 244, 232, 255))

    # Right page (creamy warm paper)
    right_page = [
        (center_x + 12, book_top + 70),
        (book_right, book_top + 30),
        (book_right, book_bottom - 10),
        (center_x + 12, book_bottom + 30)
    ]
    # Draw right page stack depth
    draw.polygon([
        (center_x + 12, book_top + 78),
        (book_right, book_top + 38),
        (book_right, book_bottom - 2),
        (center_x + 12, book_bottom + 38)
    ], fill=(215, 205, 185, 255))

    # Main Right Page
    draw.polygon(right_page, fill=(253, 250, 242, 255))

    # Book spine shadow in center
    draw.polygon([
        (center_x - 16, book_top + 60),
        (center_x + 16, book_top + 60),
        (center_x + 16, book_bottom + 45),
        (center_x - 16, book_bottom + 45)
    ], fill=(180, 165, 145, 180))

    # Text lines on left page (simulating typography)
    text_color = (160, 150, 135, 220)
    for y_offset in range(160, 440, 52):
        y1 = book_top + y_offset
        y2 = y1 + 18
        draw.rounded_rectangle([book_left + 45, y1, center_x - 55, y2], radius=8, fill=text_color)

    # Text lines on right page
    for y_offset in range(160, 440, 52):
        y1 = book_top + y_offset
        y2 = y1 + 18
        draw.rounded_rectangle([center_x + 55, y1, book_right - 45, y2], radius=8, fill=text_color)

    # Ribbon bookmark (warm carmine/gold accent)
    ribbon_color = (212, 68, 62, 255) # Carmine red
    ribbon_points = [
        (center_x - 18, book_top + 40),
        (center_x + 18, book_top + 40),
        (center_x + 18, book_bottom + 90),
        (center_x, book_bottom + 65),
        (center_x - 18, book_bottom + 90)
    ]
    draw.polygon(ribbon_points, fill=ribbon_color)

    # Save PNG at 256 for app icon
    img_256 = img.resize((256, 256), Image.Resampling.LANCZOS)
    img_256.save("assets/app_256.png")
    img.resize((64, 64), Image.Resampling.LANCZOS).save("assets/app_64.png")

    # Generate multi-size ICO
    sizes = [(16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)]
    img.save("assets/app.ico", format="ICO", sizes=sizes)
    print("Created assets/app.ico and PNGs")

def create_fb2_doc_icon():
    # Render at 1024x1024
    size = 1024
    img = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)

    # Document sheet dimensions
    doc_left = 160
    doc_right = size - 160
    doc_top = 100
    doc_bottom = size - 110
    corner_fold = 180

    # 1. Sheet drop shadow
    shadow = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    s_draw = ImageDraw.Draw(shadow)
    s_points = [
        (doc_left, doc_top + 20),
        (doc_right - corner_fold, doc_top + 20),
        (doc_right, doc_top + corner_fold + 20),
        (doc_right, doc_bottom + 20),
        (doc_left, doc_bottom + 20)
    ]
    s_draw.polygon(s_points, fill=(0, 0, 0, 70))
    img = Image.alpha_composite(shadow, img)
    draw = ImageDraw.Draw(img)

    # 2. Main paper sheet (soft warm bright paper)
    sheet_points = [
        (doc_left, doc_top),
        (doc_right - corner_fold, doc_top),
        (doc_right, doc_top + corner_fold),
        (doc_right, doc_bottom),
        (doc_left, doc_bottom)
    ]
    draw.polygon(sheet_points, fill=(252, 250, 246, 255))
    # Border
    draw.line(sheet_points + [(doc_left, doc_top)], fill=(200, 195, 185, 255), width=10)

    # 3. Folded Corner (triangle at top-right)
    fold_shadow_points = [
        (doc_right - corner_fold, doc_top),
        (doc_right - corner_fold, doc_top + corner_fold),
        (doc_right, doc_top + corner_fold)
    ]
    draw.polygon(fold_shadow_points, fill=(215, 208, 195, 255))
    draw.line([
        (doc_right - corner_fold, doc_top),
        (doc_right - corner_fold, doc_top + corner_fold),
        (doc_right, doc_top + corner_fold)
    ], fill=(175, 168, 155, 255), width=8)

    # 4. Book motif in the center upper half
    center_x = size // 2
    b_top = 280
    b_bottom = 540
    b_left = 280
    b_right = size - 280

    # Open book left page (QuietReader Sepia)
    draw.polygon([
        (b_left, b_top + 15),
        (center_x - 8, b_top + 38),
        (center_x - 8, b_bottom + 15),
        (b_left, b_bottom - 8)
    ], fill=(238, 230, 212, 255), outline=(180, 150, 120, 255), width=6)

    # Open book right page
    draw.polygon([
        (center_x + 8, b_top + 38),
        (b_right, b_top + 15),
        (b_right, b_bottom - 8),
        (center_x + 8, b_bottom + 15)
    ], fill=(244, 237, 222, 255), outline=(180, 150, 120, 255), width=6)

    # Text lines inside book emblem
    for yo in range(70, 220, 38):
        draw.line([(b_left + 25, b_top + yo), (center_x - 30, b_top + yo + 14)], fill=(180, 165, 145, 255), width=6)
        draw.line([(center_x + 30, b_top + yo + 14), (b_right - 25, b_top + yo)], fill=(180, 165, 145, 255), width=6)

    # Center bookmark ribbon
    draw.polygon([
        (center_x - 10, b_top + 20),
        (center_x + 10, b_top + 20),
        (center_x + 10, b_bottom + 45),
        (center_x, b_bottom + 30),
        (center_x - 10, b_bottom + 45)
    ], fill=(212, 68, 62, 255))

    # 5. Document text placeholder lines
    line_y = 600
    draw.rounded_rectangle([doc_left + 80, line_y, doc_right - 80, line_y + 12], radius=6, fill=(215, 210, 200, 255))
    draw.rounded_rectangle([doc_left + 80, line_y + 36, doc_right - 140, line_y + 48], radius=6, fill=(225, 220, 212, 255))

    # 6. Prominent "FB2" Badge at the bottom
    badge_left = doc_left + 45
    badge_right = doc_right - 45
    badge_top = 700
    badge_bottom = 850
    badge_radius = 28

    # Badge background: deep QuietReader navy
    draw.rounded_rectangle(
        [badge_left, badge_top, badge_right, badge_bottom],
        radius=badge_radius,
        fill=(28, 45, 74, 255),
        outline=(44, 72, 118, 255),
        width=8
    )

    # Draw "FB2" text in bold on the badge
    # Try finding Arial or standard system font, or draw custom geometric letters
    font = None
    for font_path in [
        "C:\\Windows\\Fonts\\arialbd.ttf",
        "C:\\Windows\\Fonts\\segouib.ttf",
        "C:\\Windows\\Fonts\\tahomabd.ttf",
        "C:\\Windows\\Fonts\\arial.ttf"
    ]:
        if os.path.exists(font_path):
            try:
                font = ImageFont.truetype(font_path, 115)
                break
            except Exception:
                pass

    if font:
        text = "FB2"
        bbox = draw.textbbox((0, 0), text, font=font)
        w = bbox[2] - bbox[0]
        h = bbox[3] - bbox[1]
        x = (size - w) // 2
        y = badge_top + (badge_bottom - badge_top - h) // 2 - 8
        draw.text((x, y), text, font=font, fill=(255, 255, 255, 255))
    else:
        # Fallback if no TTF
        draw.text((center_x - 80, badge_top + 40), "FB2", fill=(255, 255, 255, 255))

    # Save PNG and ICO
    img_256 = img.resize((256, 256), Image.Resampling.LANCZOS)
    img_256.save("assets/fb2_doc_256.png")
    
    sizes = [(16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)]
    img.save("assets/fb2_doc.ico", format="ICO", sizes=sizes)
    print("Created assets/fb2_doc.ico and PNGs")

if __name__ == "__main__":
    create_app_icon()
    create_fb2_doc_icon()
