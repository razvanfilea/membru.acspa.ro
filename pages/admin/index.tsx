import {Button, Card, Group, Paper, Text} from "@mantine/core";
import Link from "next/link";

export default function AdminPage() {
    return <Paper>
        <Card sx={(theme) => ({margin: theme.spacing.md})}>
            <Text size={'xl'}>Panou fondatori</Text>
            <Text size={'sm'} sx={(theme) => ({margin: theme.spacing.xs})}>
                Este necesară discreția utilizatorului: datele de intrare nu sunt validate.<br/>
                Dezvoltatorul nu își asumă nicio responsabilitate pentru această parte a aplicației!</Text>
            <Group>
                <Link href={'/admin/restrictions'}>
                    <Button>Restricționare rezervări</Button>
                </Link>
                <Link href={'/admin/guests'}>
                    <Button>Adăugare invitați</Button>
                </Link>
            </Group>
        </Card>
    </Paper>
}
