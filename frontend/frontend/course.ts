import { marked } from 'marked';
import './style.css';

document.addEventListener('DOMContentLoaded', async () => {
    const courseData = localStorage.getItem('currentCourse');
    if (!courseData) {
        window.location.href = '/';
        return;
    }
    
    const contentContainer = document.getElementById('course-content') as HTMLElement;
    const tocList = document.getElementById('toc-list') as HTMLElement;
    
    try {
        // Render Markdown using the imported 'marked' library
        const renderedHtml = await marked.parse(courseData);
        contentContainer.innerHTML = renderedHtml;
    } catch (error) {
        console.error('Error parsing markdown:', error);
        contentContainer.innerHTML = '<div class="error">Failed to render course content. Please try again.</div>';
    }
    
    // Generate TOC from h2 and h3 elements
    const headings = contentContainer.querySelectorAll('h2, h3');
    if (headings.length === 0) {
        const sidebar = document.querySelector('.course-sidebar') as HTMLElement;
        const layout = document.querySelector('.course-layout') as HTMLElement;
        if (sidebar) sidebar.style.display = 'none';
        if (layout) layout.style.justifyContent = 'center';
    } else {
        headings.forEach((heading, index) => {
            const id = 'section-' + index;
            heading.id = id;
            
            const li = document.createElement('li');
            const a = document.createElement('a');
            a.href = '#' + id;
            a.textContent = heading.textContent;
            a.classList.add('toc-link');
            
            if (heading.tagName.toLowerCase() === 'h3') {
                a.classList.add('toc-sub-link');
            }
            
            // Simple active state on click
            a.addEventListener('click', () => {
                document.querySelectorAll('.toc-link').forEach(l => l.classList.remove('active'));
                a.classList.add('active');
            });

            li.appendChild(a);
            tocList.appendChild(li);
        });
    }
});
